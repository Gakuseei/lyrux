%global debug_package %{nil}

Name:       lyrux
Version:    %{version}
Release:    1%{?dist}
Summary:    GTK4 file manager + terminal multiplexer for Linux
License:    MIT
URL:        https://github.com/Gakuseei/lyrux
Vendor:     Gakuseei <erikschaefer07@icloud.com>
ExclusiveArch: x86_64 aarch64
AutoReq:    yes
Source0:    lyrux-%{version}.tar.gz

%description
Lyrux is a GTK4 file manager and terminal multiplexer powered by Ghostty's
GPU-rendered terminal engine, with split panes, tabbed workspaces, and a
built-in browser.

%prep
%setup -q

%build

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a %{_builddir}/lyrux-%{version}/usr %{buildroot}/
cp -a %{_builddir}/lyrux-%{version}/etc %{buildroot}/

%post
ldconfig 2>/dev/null || true
rm -f %{_datadir}/applications/lyrux.desktop
gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor 2>/dev/null || true
update-desktop-database %{_datadir}/applications 2>/dev/null || true
appstreamcli refresh-cache --force 2>/dev/null || true

%postun
ldconfig 2>/dev/null || true
gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor 2>/dev/null || true
update-desktop-database %{_datadir}/applications 2>/dev/null || true
appstreamcli refresh-cache --force 2>/dev/null || true

%files
%{_bindir}/lyrux
/usr/lib/lyrux/libghostty.so
%{_datadir}/lyrux/
%{_datadir}/applications/dev.lyrux.linux.desktop
%{_datadir}/metainfo/dev.lyrux.linux.metainfo.xml
%{_datadir}/icons/hicolor/
%{_sysconfdir}/ld.so.conf.d/lyrux.conf

%changelog
