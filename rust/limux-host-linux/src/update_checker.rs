use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const RELEASES_URL: &str = "https://github.com/am-will/limux/releases";
const LATEST_RELEASE_API_URL: &str = "https://api.github.com/repos/am-will/limux/releases/latest";
const UPDATE_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallMode {
    Bundle,
    TarballPrefix,
    AppImage,
    DebianPackage,
    RpmPackage,
    Unsupported,
}

impl InstallMode {
    pub fn label(self) -> &'static str {
        match self {
            InstallMode::Bundle => "portable bundle",
            InstallMode::TarballPrefix => "tarball install",
            InstallMode::AppImage => "AppImage",
            InstallMode::DebianPackage => "Debian package",
            InstallMode::RpmPackage => "RPM package",
            InstallMode::Unsupported => "unsupported installation",
        }
    }

    pub fn needs_privileged_installer(self) -> bool {
        matches!(
            self,
            InstallMode::TarballPrefix | InstallMode::DebianPackage | InstallMode::RpmPackage
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallTarget {
    pub mode: InstallMode,
    pub target_path: PathBuf,
    pub relaunch_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub digest_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    UpToDate,
    UpdateAvailable,
    UnsupportedInstallation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateSummary {
    pub current_version: String,
    pub latest_version: String,
    pub latest_tag: String,
    pub published_at: String,
    pub commits_behind: u64,
    pub release_url: String,
    pub selected_asset: Option<ReleaseAsset>,
    pub install_target: InstallTarget,
    pub status: UpdateStatus,
    pub status_detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedUpdate {
    pub manifest_path: PathBuf,
    pub asset_name: String,
    pub latest_tag: String,
    pub install_mode: InstallMode,
}

#[derive(Debug, Deserialize)]
struct LatestReleaseResponse {
    tag_name: String,
    html_url: String,
    published_at: String,
    assets: Vec<ReleaseAssetResponse>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAssetResponse {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompareResponse {
    total_commits: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateManifest {
    wait_for_pid: u32,
    release_tag: String,
    release_url: String,
    install_target: InstallTarget,
    download_path: PathBuf,
    work_dir: PathBuf,
}

pub fn fetch_update_summary(current_version: &str) -> Result<UpdateSummary, String> {
    let agent = build_http_agent();
    let latest = fetch_latest_release(&agent, current_version)?;
    build_update_summary(current_version, latest, &agent)
}

pub fn prepare_update(summary: &UpdateSummary) -> Result<PreparedUpdate, String> {
    if summary.status != UpdateStatus::UpdateAvailable {
        return Err("No installable update is currently available.".to_string());
    }

    let asset = summary.selected_asset.clone().ok_or_else(|| {
        "The latest release does not provide a matching installer asset.".to_string()
    })?;

    let work_dir = update_work_dir()?;
    let download_path = work_dir.join(&asset.name);
    download_asset(&asset, &download_path)?;
    verify_download_digest(&asset, &download_path)?;

    let manifest = UpdateManifest {
        wait_for_pid: std::process::id(),
        release_tag: summary.latest_tag.clone(),
        release_url: summary.release_url.clone(),
        install_target: summary.install_target.clone(),
        download_path,
        work_dir: work_dir.clone(),
    };

    let manifest_path = work_dir.join("manifest.json");
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|err| format!("Failed to write update manifest: {err}"))?;
    fs::write(&manifest_path, manifest_bytes)
        .map_err(|err| format!("Failed to persist the update manifest: {err}"))?;

    Ok(PreparedUpdate {
        manifest_path,
        asset_name: asset.name,
        latest_tag: summary.latest_tag.clone(),
        install_mode: summary.install_target.mode,
    })
}

pub fn spawn_update_helper(manifest_path: &Path) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("Failed to resolve the current Limux executable: {err}"))?;

    Command::new(exe)
        .arg("--apply-update")
        .arg(manifest_path)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Failed to launch the Limux update helper: {err}"))
}

pub fn apply_prepared_update_from_manifest(manifest_path: &Path) -> Result<(), String> {
    let manifest_bytes =
        fs::read(manifest_path).map_err(|err| format!("Failed to read update manifest: {err}"))?;
    let manifest: UpdateManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| format!("Failed to parse update manifest: {err}"))?;

    wait_for_process_exit(manifest.wait_for_pid, UPDATE_WAIT_TIMEOUT)?;

    match manifest.install_target.mode {
        InstallMode::Bundle => apply_bundle_update(&manifest)?,
        InstallMode::TarballPrefix => apply_tarball_prefix_update(&manifest)?,
        InstallMode::AppImage => apply_appimage_update(&manifest)?,
        InstallMode::DebianPackage => apply_package_update(&manifest, "dpkg", &["-i"])?,
        InstallMode::RpmPackage => apply_package_update(&manifest, "rpm", &["-Uvh"])?,
        InstallMode::Unsupported => {
            return Err("This Limux installation cannot be updated in place.".to_string());
        }
    }

    cleanup_update_work_dir(&manifest.work_dir);
    relaunch_limux(&manifest.install_target.relaunch_path)?;
    Ok(())
}

pub fn format_release_date(value: &str) -> String {
    value.split('T').next().unwrap_or(value).to_string()
}

fn build_http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
}

fn fetch_latest_release(
    agent: &ureq::Agent,
    current_version: &str,
) -> Result<LatestReleaseResponse, String> {
    read_json::<LatestReleaseResponse>(
        agent,
        LATEST_RELEASE_API_URL,
        current_version,
        "Failed to fetch the latest Limux release",
    )
}

fn fetch_compare_commits(
    agent: &ureq::Agent,
    current_tag: &str,
    latest_tag: &str,
    current_version: &str,
) -> Result<u64, String> {
    let url =
        format!("https://api.github.com/repos/am-will/limux/compare/{current_tag}...{latest_tag}");
    let compare = read_json::<CompareResponse>(
        agent,
        &url,
        current_version,
        "Failed to compare the installed Limux version with the latest release",
    )?;
    Ok(compare.total_commits)
}

fn read_json<T: for<'de> Deserialize<'de>>(
    agent: &ureq::Agent,
    url: &str,
    current_version: &str,
    context: &str,
) -> Result<T, String> {
    let response = agent
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", &format!("limux/{current_version}"))
        .call()
        .map_err(|err| format_http_error(context, err))?;

    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .map_err(|err| format!("{context}: {err}"))?;

    serde_json::from_str(&body).map_err(|err| format!("{context}: {err}"))
}

fn build_update_summary(
    current_version: &str,
    latest: LatestReleaseResponse,
    agent: &ureq::Agent,
) -> Result<UpdateSummary, String> {
    let install_target = resolve_install_target()?;
    let selected_asset = select_release_asset(&latest, install_target.mode);

    if install_target.mode == InstallMode::Unsupported {
        return Ok(UpdateSummary {
            current_version: current_version.to_string(),
            latest_version: strip_version_prefix(&latest.tag_name).to_string(),
            latest_tag: latest.tag_name,
            published_at: latest.published_at,
            commits_behind: 0,
            release_url: latest.html_url,
            selected_asset: None,
            install_target,
            status: UpdateStatus::UnsupportedInstallation,
            status_detail: "This Limux installation type is not recognized for in-app updates."
                .to_string(),
        });
    }

    let Some(selected_asset) = selected_asset else {
        let mode_label = install_target.mode.label().to_string();
        return Ok(UpdateSummary {
            current_version: current_version.to_string(),
            latest_version: strip_version_prefix(&latest.tag_name).to_string(),
            latest_tag: latest.tag_name,
            published_at: latest.published_at,
            commits_behind: 0,
            release_url: latest.html_url,
            selected_asset: None,
            install_target,
            status: UpdateStatus::UnsupportedInstallation,
            status_detail: format!(
                "No {} update asset is available for this release on {}.",
                mode_label,
                std::env::consts::ARCH
            ),
        });
    };

    let current_tag = version_tag(current_version);
    let version_order = compare_release_versions(current_version, &latest.tag_name);

    let (status, commits_behind, status_detail) = match version_order {
        Ordering::Less => {
            let commits_behind =
                fetch_compare_commits(agent, &current_tag, &latest.tag_name, current_version)?;
            let label = if commits_behind == 1 {
                "commit"
            } else {
                "commits"
            };
            let installer_hint = if install_target.mode.needs_privileged_installer() {
                " Limux may ask for administrator permission during install."
            } else {
                ""
            };
            (
                UpdateStatus::UpdateAvailable,
                commits_behind,
                format!(
                    "Update available: {commits_behind} {label} behind. Installer: {} via {}.{}",
                    selected_asset.name,
                    install_target.mode.label(),
                    installer_hint
                ),
            )
        }
        Ordering::Equal => (
            UpdateStatus::UpToDate,
            0,
            "This build matches the latest release.".to_string(),
        ),
        Ordering::Greater => (
            UpdateStatus::UpToDate,
            0,
            "This build is newer than the latest published release.".to_string(),
        ),
    };

    Ok(UpdateSummary {
        current_version: current_version.to_string(),
        latest_version: strip_version_prefix(&latest.tag_name).to_string(),
        latest_tag: latest.tag_name,
        published_at: latest.published_at,
        commits_behind,
        release_url: latest.html_url,
        selected_asset: Some(selected_asset),
        install_target,
        status,
        status_detail,
    })
}

fn resolve_install_target() -> Result<InstallTarget, String> {
    let current_exe = std::env::current_exe()
        .map_err(|err| format!("Failed to resolve the current Limux executable: {err}"))?;
    Ok(resolve_install_target_for(
        &current_exe,
        std::env::var_os("APPIMAGE").as_deref(),
    ))
}

fn resolve_install_target_for(
    current_exe: &Path,
    appimage: Option<&std::ffi::OsStr>,
) -> InstallTarget {
    if let Some(appimage_path) = appimage.filter(|value| !value.is_empty()) {
        let target_path = PathBuf::from(appimage_path);
        return InstallTarget {
            mode: InstallMode::AppImage,
            target_path: target_path.clone(),
            relaunch_path: target_path,
        };
    }

    if current_exe
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("AppImage"))
    {
        return InstallTarget {
            mode: InstallMode::AppImage,
            target_path: current_exe.to_path_buf(),
            relaunch_path: current_exe.to_path_buf(),
        };
    }

    if let Some(root) = current_exe
        .parent()
        .filter(|root| is_portable_bundle_root(root))
    {
        return InstallTarget {
            mode: InstallMode::Bundle,
            target_path: root.to_path_buf(),
            relaunch_path: current_exe.to_path_buf(),
        };
    }

    if current_exe.starts_with("/usr/local/bin/") || current_exe.starts_with("/usr/local/bin") {
        return InstallTarget {
            mode: InstallMode::TarballPrefix,
            target_path: PathBuf::from("/usr/local"),
            relaunch_path: current_exe.to_path_buf(),
        };
    }

    if current_exe.starts_with("/usr/bin/") || current_exe.starts_with("/usr/bin") {
        let mode = preferred_package_install_mode();
        return InstallTarget {
            mode,
            target_path: current_exe.to_path_buf(),
            relaunch_path: current_exe.to_path_buf(),
        };
    }

    InstallTarget {
        mode: InstallMode::Unsupported,
        target_path: current_exe.to_path_buf(),
        relaunch_path: current_exe.to_path_buf(),
    }
}

fn preferred_package_install_mode() -> InstallMode {
    if Path::new("/etc/debian_version").exists() || command_exists("dpkg") {
        InstallMode::DebianPackage
    } else if Path::new("/etc/redhat-release").exists() || command_exists("rpm") {
        InstallMode::RpmPackage
    } else {
        InstallMode::Unsupported
    }
}

fn select_release_asset(
    latest: &LatestReleaseResponse,
    install_mode: InstallMode,
) -> Option<ReleaseAsset> {
    let expected_name = asset_name_for(install_mode, strip_version_prefix(&latest.tag_name))?;
    latest
        .assets
        .iter()
        .find(|asset| asset.name == expected_name)
        .map(|asset| ReleaseAsset {
            name: asset.name.clone(),
            download_url: asset.browser_download_url.clone(),
            digest_sha256: parse_sha256_digest(asset.digest.as_deref()),
        })
}

fn asset_name_for(install_mode: InstallMode, version: &str) -> Option<String> {
    let arch = std::env::consts::ARCH;

    match install_mode {
        InstallMode::Bundle | InstallMode::TarballPrefix => {
            Some(format!("limux-{version}-linux-{arch}.tar.gz"))
        }
        InstallMode::AppImage => Some(format!("Limux-{version}-{arch}.AppImage")),
        InstallMode::DebianPackage => match arch {
            "x86_64" => Some(format!("limux_{version}_amd64.deb")),
            "aarch64" => Some(format!("limux_{version}_arm64.deb")),
            _ => None,
        },
        InstallMode::RpmPackage => match arch {
            "x86_64" => Some(format!("limux-{version}-1.x86_64.rpm")),
            "aarch64" => Some(format!("limux-{version}-1.aarch64.rpm")),
            _ => None,
        },
        InstallMode::Unsupported => None,
    }
}

fn update_work_dir() -> Result<PathBuf, String> {
    let base = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("limux")
        .join("updates");
    let work_dir = base.join(Uuid::new_v4().to_string());
    fs::create_dir_all(&work_dir)
        .map_err(|err| format!("Failed to create the Limux update cache directory: {err}"))?;
    Ok(work_dir)
}

fn download_asset(asset: &ReleaseAsset, destination: &Path) -> Result<(), String> {
    let agent = build_http_agent();
    let response = agent
        .get(&asset.download_url)
        .set("Accept", "application/octet-stream")
        .set("User-Agent", &format!("limux/{}", crate::VERSION))
        .call()
        .map_err(|err| format_http_error("Failed to download the update asset", err))?;

    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination)
        .map_err(|err| format!("Failed to create the update download file: {err}"))?;
    io::copy(&mut reader, &mut file)
        .map_err(|err| format!("Failed to write the downloaded update asset: {err}"))?;
    file.flush()
        .map_err(|err| format!("Failed to flush the downloaded update asset: {err}"))?;
    Ok(())
}

fn verify_download_digest(asset: &ReleaseAsset, path: &Path) -> Result<(), String> {
    let Some(expected) = asset.digest_sha256.as_deref() else {
        return Ok(());
    };

    let actual = sha256_for_file(path)?;
    if actual != expected {
        return Err(format!(
            "Downloaded asset checksum mismatch for {}: expected {expected}, got {actual}.",
            asset.name
        ));
    }

    Ok(())
}

fn sha256_for_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|err| format!("Failed to open the downloaded asset: {err}"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let bytes = file
            .read(&mut buffer)
            .map_err(|err| format!("Failed to read the downloaded asset: {err}"))?;
        if bytes == 0 {
            break;
        }
        digest.update(&buffer[..bytes]);
    }

    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn apply_bundle_update(manifest: &UpdateManifest) -> Result<(), String> {
    let bundle_root = &manifest.install_target.target_path;
    let extract_root = manifest.work_dir.join("extract");
    fs::create_dir_all(&extract_root)
        .map_err(|err| format!("Failed to prepare the bundle extraction directory: {err}"))?;
    extract_tarball(&manifest.download_path, &extract_root)?;

    let extracted_root = extracted_payload_root(&extract_root)?;
    let staging_root = bundle_root.join(format!(".limux-update-stage-{}", Uuid::new_v4()));
    fs::create_dir_all(&staging_root)
        .map_err(|err| format!("Failed to prepare the bundle staging directory: {err}"))?;

    for entry in ["lib", "share"] {
        let src = extracted_root.join(entry);
        if src.exists() {
            copy_tree(&src, &staging_root.join(entry))?;
        }
    }
    for entry in ["limux", "install.sh"] {
        let src = extracted_root.join(entry);
        if src.exists() {
            copy_file_with_mode(&src, &staging_root.join(entry))?;
        }
    }

    swap_directory(&staging_root.join("lib"), &bundle_root.join("lib"))?;
    swap_directory(&staging_root.join("share"), &bundle_root.join("share"))?;
    replace_file_within_root(&staging_root.join("limux"), &bundle_root.join("limux"))?;
    replace_file_within_root(
        &staging_root.join("install.sh"),
        &bundle_root.join("install.sh"),
    )?;

    let _ = fs::remove_dir_all(&staging_root);
    Ok(())
}

fn apply_tarball_prefix_update(manifest: &UpdateManifest) -> Result<(), String> {
    let extract_root = manifest.work_dir.join("extract");
    fs::create_dir_all(&extract_root)
        .map_err(|err| format!("Failed to prepare the tarball extraction directory: {err}"))?;
    extract_tarball(&manifest.download_path, &extract_root)?;

    let extracted_root = extracted_payload_root(&extract_root)?;
    let install_script = extracted_root.join("install.sh");
    if !install_script.is_file() {
        return Err("The downloaded tarball does not contain an install.sh script.".to_string());
    }

    run_install_command(
        &install_script,
        &[OsString::from(format!(
            "--prefix={}",
            manifest.install_target.target_path.display()
        ))],
    )
}

fn apply_appimage_update(manifest: &UpdateManifest) -> Result<(), String> {
    let target = &manifest.install_target.target_path;
    let staged = target.with_extension(format!("{}.download", Uuid::new_v4()));
    copy_file_with_mode(&manifest.download_path, &staged)?;

    let mut permissions = fs::metadata(&staged)
        .map_err(|err| format!("Failed to inspect the staged AppImage: {err}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&staged, permissions)
        .map_err(|err| format!("Failed to mark the staged AppImage as executable: {err}"))?;

    fs::rename(&staged, target)
        .map_err(|err| format!("Failed to replace the existing AppImage with the update: {err}"))?;
    Ok(())
}

fn apply_package_update(
    manifest: &UpdateManifest,
    command: &str,
    args: &[&str],
) -> Result<(), String> {
    if !command_exists(command) {
        return Err(format!(
            "Cannot install the update because `{command}` is not available."
        ));
    }

    let mut command_args = args
        .iter()
        .map(|arg| OsString::from(*arg))
        .collect::<Vec<_>>();
    command_args.push(manifest.download_path.clone().into_os_string());
    run_privileged_command(command, &command_args)
}

fn run_install_command(script: &Path, args: &[OsString]) -> Result<(), String> {
    if is_writable_by_current_user(script.parent().unwrap_or_else(|| Path::new("/"))) {
        let status = Command::new(script)
            .args(args)
            .status()
            .map_err(|err| format!("Failed to run the Limux installer: {err}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!(
            "The Limux installer exited with status {}.",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ));
    }

    let mut full_args = vec![script.as_os_str().to_os_string()];
    full_args.extend_from_slice(args);
    run_privileged_command(script, &full_args)
}

fn run_privileged_command(program: impl AsRef<Path>, args: &[OsString]) -> Result<(), String> {
    if command_exists("pkexec") {
        run_command(
            "pkexec",
            std::iter::once(program.as_ref().as_os_str().to_os_string())
                .chain(args.iter().cloned()),
        )
    } else if io::stdin().is_terminal() && command_exists("sudo") {
        run_command(
            "sudo",
            std::iter::once(program.as_ref().as_os_str().to_os_string())
                .chain(args.iter().cloned()),
        )
    } else {
        Err(
            "Administrator permission is required for this update, but Limux could not find `pkexec` and no terminal-backed `sudo` session is available."
                .to_string(),
        )
    }
}

fn run_command(program: &str, args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|err| format!("Failed to launch `{program}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{program}` exited with status {}.",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ))
    }
}

fn extract_tarball(tarball: &Path, destination: &Path) -> Result<(), String> {
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(tarball)
        .arg("-C")
        .arg(destination)
        .status()
        .map_err(|err| format!("Failed to launch `tar` to unpack the update: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`tar` exited with status {} while unpacking the update.",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ))
    }
}

fn extracted_payload_root(root: &Path) -> Result<PathBuf, String> {
    let mut dirs = fs::read_dir(root)
        .map_err(|err| format!("Failed to inspect the extracted update payload: {err}"))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    dirs.sort();
    dirs.into_iter().next().ok_or_else(|| {
        "The extracted update payload is missing its top-level directory.".to_string()
    })
}

fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(src)
        .map_err(|err| format!("Failed to inspect `{}`: {err}", src.display()))?;

    if metadata.file_type().is_symlink() {
        let target = fs::read_link(src)
            .map_err(|err| format!("Failed to read symlink `{}`: {err}", src.display()))?;
        let _ = fs::remove_file(dst);
        symlink(target, dst)
            .map_err(|err| format!("Failed to recreate symlink `{}`: {err}", dst.display()))?;
        return Ok(());
    }

    if metadata.is_dir() {
        fs::create_dir_all(dst)
            .map_err(|err| format!("Failed to create directory `{}`: {err}", dst.display()))?;
        for entry in fs::read_dir(src)
            .map_err(|err| format!("Failed to read directory `{}`: {err}", src.display()))?
        {
            let entry =
                entry.map_err(|err| format!("Failed to inspect `{}`: {err}", src.display()))?;
            copy_tree(&entry.path(), &dst.join(entry.file_name()))?;
        }
        fs::set_permissions(dst, metadata.permissions()).map_err(|err| {
            format!(
                "Failed to preserve permissions on `{}`: {err}",
                dst.display()
            )
        })?;
        return Ok(());
    }

    copy_file_with_mode(src, dst)
}

fn copy_file_with_mode(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create directory `{}`: {err}", parent.display()))?;
    }
    fs::copy(src, dst).map_err(|err| {
        format!(
            "Failed to copy `{}` to `{}`: {err}",
            src.display(),
            dst.display()
        )
    })?;

    let permissions = fs::metadata(src)
        .map_err(|err| format!("Failed to inspect `{}`: {err}", src.display()))?
        .permissions();
    fs::set_permissions(dst, permissions)
        .map_err(|err| format!("Failed to set permissions on `{}`: {err}", dst.display()))
}

fn swap_directory(staged: &Path, target: &Path) -> Result<(), String> {
    if !staged.exists() {
        return Ok(());
    }

    let backup = target.with_extension(format!("limux-backup-{}", Uuid::new_v4()));
    if target.exists() {
        fs::rename(target, &backup).map_err(|err| {
            format!(
                "Failed to move the existing directory `{}` out of the way: {err}",
                target.display()
            )
        })?;
    }

    fs::rename(staged, target).map_err(|err| {
        format!(
            "Failed to move the staged directory `{}` into place: {err}",
            target.display()
        )
    })?;

    let _ = fs::remove_dir_all(backup);
    Ok(())
}

fn replace_file_within_root(staged: &Path, target: &Path) -> Result<(), String> {
    if !staged.exists() {
        return Ok(());
    }

    fs::rename(staged, target).map_err(|err| {
        format!(
            "Failed to replace `{}` with the downloaded update: {err}",
            target.display()
        )
    })
}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> Result<(), String> {
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    let started = Instant::now();

    while proc_path.exists() {
        if started.elapsed() > timeout {
            return Err(format!(
                "Timed out waiting for the running Limux process ({pid}) to exit before installing the update."
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn relaunch_limux(path: &Path) -> Result<(), String> {
    let mut command = Command::new(path);
    command.env_remove("APPIMAGE");
    command.env_remove("APPDIR");
    command
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Updated successfully, but failed to relaunch Limux: {err}"))
}

fn cleanup_update_work_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn parse_sha256_digest(value: Option<&str>) -> Option<String> {
    value?
        .strip_prefix("sha256:")
        .map(|digest| digest.trim().to_ascii_lowercase())
}

fn format_http_error(context: &str, error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, response) => {
            if code == 403 || code == 429 {
                let remaining = response.header("x-ratelimit-remaining");
                if remaining == Some("0") {
                    let wait = response
                        .header("x-ratelimit-reset")
                        .and_then(|v| v.parse::<u64>().ok())
                        .and_then(|reset| {
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .ok()
                                .map(|now| reset.saturating_sub(now.as_secs()))
                        })
                        .map(|secs| secs.div_ceil(60).max(1));
                    return match wait {
                        Some(mins) => format!(
                            "{context}: GitHub API rate limit reached (60/h per IP). Try again in ~{mins} min."
                        ),
                        None => format!(
                            "{context}: GitHub API rate limit reached (60/h per IP). Try again later."
                        ),
                    };
                }
            }
            let status_text = response.status_text().to_string();
            format!("{context}: GitHub returned HTTP {code} {status_text}")
        }
        ureq::Error::Transport(error) => format!("{context}: {error}"),
    }
}

fn is_portable_bundle_root(path: &Path) -> bool {
    path.join("limux").is_file() && path.join("lib").is_dir() && path.join("share").is_dir()
}

fn is_writable_by_current_user(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o200 != 0)
        .unwrap_or(false)
}

fn command_exists(name: &str) -> bool {
    let path = Path::new(name);
    if path.components().count() > 1 {
        return path.is_file();
    }

    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|entry| entry.join(name).is_file()))
        .unwrap_or(false)
}

fn version_tag(version: &str) -> String {
    let stripped = strip_version_prefix(version);
    format!("v{stripped}")
}

fn compare_release_versions(left: &str, right: &str) -> Ordering {
    match (version_components(left), version_components(right)) {
        (Some(left), Some(right)) => {
            let max_len = left.len().max(right.len());
            for idx in 0..max_len {
                let left_part = left.get(idx).copied().unwrap_or(0);
                let right_part = right.get(idx).copied().unwrap_or(0);
                match left_part.cmp(&right_part) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            Ordering::Equal
        }
        _ => strip_version_prefix(left).cmp(strip_version_prefix(right)),
    }
}

fn version_components(value: &str) -> Option<Vec<u32>> {
    let trimmed = strip_version_prefix(value);
    let core = trimmed.split('-').next().unwrap_or(trimmed);
    core.split('.')
        .map(|part| part.parse::<u32>().ok())
        .collect::<Option<Vec<_>>>()
}

fn strip_version_prefix(value: &str) -> &str {
    value.trim().strip_prefix('v').unwrap_or(value.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_release_versions_handles_v_prefix() {
        assert_eq!(
            compare_release_versions("0.1.13", "v0.1.13"),
            Ordering::Equal
        );
        assert_eq!(
            compare_release_versions("0.1.12", "v0.1.13"),
            Ordering::Less
        );
        assert_eq!(
            compare_release_versions("v0.2.0", "0.1.13"),
            Ordering::Greater
        );
    }

    #[test]
    fn format_release_date_uses_calendar_date_prefix() {
        assert_eq!(format_release_date("2026-04-12T06:50:45Z"), "2026-04-12");
        assert_eq!(format_release_date("2026-04-12"), "2026-04-12");
    }

    #[test]
    fn build_update_summary_marks_current_release_as_up_to_date() {
        let latest = LatestReleaseResponse {
            tag_name: "v0.1.13".to_string(),
            html_url: "https://github.com/am-will/limux/releases/tag/v0.1.13".to_string(),
            published_at: "2026-04-12T06:50:45Z".to_string(),
            assets: vec![ReleaseAssetResponse {
                name: "limux-0.1.13-linux-x86_64.tar.gz".to_string(),
                browser_download_url: "https://example.invalid/limux-0.1.13-linux-x86_64.tar.gz"
                    .to_string(),
                digest: Some(
                    "sha256:76ce9b070b68437bf570fe4d3a5be99b46bfb9e9110c1b832c6667b871920fff"
                        .to_string(),
                ),
            }],
        };
        let agent = build_http_agent();

        let summary = build_update_summary("0.1.13", latest, &agent).unwrap();

        assert!(matches!(
            summary.status,
            UpdateStatus::UpToDate | UpdateStatus::UnsupportedInstallation
        ));
        assert_eq!(summary.latest_version, "0.1.13");
    }

    #[test]
    fn version_tag_normalizes_existing_prefix() {
        assert_eq!(version_tag("0.1.13"), "v0.1.13");
        assert_eq!(version_tag("v0.1.13"), "v0.1.13");
    }

    #[test]
    fn parse_sha256_digest_strips_prefix() {
        assert_eq!(
            parse_sha256_digest(Some("sha256:ABCDEF")),
            Some("abcdef".to_string())
        );
        assert_eq!(parse_sha256_digest(None), None);
    }

    #[test]
    fn select_release_asset_uses_install_mode_and_arch() {
        let release = LatestReleaseResponse {
            tag_name: "v0.1.13".to_string(),
            html_url: "https://example.invalid".to_string(),
            published_at: "2026-04-12T06:50:45Z".to_string(),
            assets: vec![
                ReleaseAssetResponse {
                    name: format!("limux-0.1.13-linux-{}.tar.gz", std::env::consts::ARCH),
                    browser_download_url: "https://example.invalid/asset.tar.gz".to_string(),
                    digest: None,
                },
                ReleaseAssetResponse {
                    name: "limux_0.1.13_amd64.deb".to_string(),
                    browser_download_url: "https://example.invalid/asset.deb".to_string(),
                    digest: None,
                },
            ],
        };

        let asset = select_release_asset(&release, InstallMode::Bundle).unwrap();
        assert!(asset.name.ends_with(".tar.gz"));
    }

    #[test]
    fn resolve_install_target_detects_appimage_from_env() {
        let target = resolve_install_target_for(
            Path::new("/tmp/.mount-limux/usr/bin/limux"),
            Some(std::ffi::OsStr::new("/home/test/Limux.AppImage")),
        );

        assert_eq!(target.mode, InstallMode::AppImage);
        assert_eq!(
            target.target_path,
            PathBuf::from("/home/test/Limux.AppImage")
        );
    }
}
