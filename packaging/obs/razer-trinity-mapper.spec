Name:           razer-trinity-mapper
Version:        0.2.2
Release:        1%{?dist}
Summary:        Razer Naga Trinity 12-button remapper for Linux

License:        GPL-3.0-or-later
URL:            https://github.com/4lador/razer-trinity-mapper
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  pkgconfig(xkbcommon)
%{!?_udevrulesdir: %global _udevrulesdir %{_prefix}/lib/udev/rules.d}
%{!?_userunitdir: %global _userunitdir %{_prefix}/lib/systemd/user}
ExclusiveArch:  x86_64

%description
Remap the 12 side buttons of the Razer Naga Trinity to keyboard keys at
the kernel level (evdev grab + uinput injection). Includes an Iced GUI,
a headless daemon started at login and a CLI (trinity-ctl).

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --offline --frozen

%install
install -Dm755 target/release/trinity-daemon %{buildroot}%{_bindir}/trinity-daemon
install -Dm755 target/release/trinity-gui   %{buildroot}%{_bindir}/trinity-gui
install -Dm755 target/release/trinity-ctl   %{buildroot}%{_bindir}/trinity-ctl
install -Dm644 packaging/60-trinity-mapper.rules \
    %{buildroot}%{_udevrulesdir}/60-trinity-mapper.rules
install -Dm644 packaging/trinity-mapper-system.service \
    %{buildroot}%{_userunitdir}/trinity-mapper.service

%files
%license LICENSE
%{_bindir}/trinity-daemon
%{_bindir}/trinity-gui
%{_bindir}/trinity-ctl
%{_udevrulesdir}/60-trinity-mapper.rules
%{_userunitdir}/trinity-mapper.service

%changelog
