# TUIX — Raspberry Pi Setup Guide

Complete guide to setting up TUIX on a Raspberry Pi with the 7" ROADOM touchscreen (1024×600).

---

## 1. What You Need

- Raspberry Pi 4 Model B (2GB+ RAM) or Raspberry Pi 5
- microSD card (16GB minimum, 32GB recommended)
- ROADOM 7" IPS Touch Screen (1024×600, HDMI)
- USB-C power supply (5V 3A for Pi 4, 5V 5A for Pi 5)
- USB cable from the touchscreen to the Pi (for touch input)
- HDMI cable (micro-HDMI to standard HDMI for Pi 4, or full HDMI for Pi 5)
- A keyboard for initial setup (can be removed after)
- Internet connection (Ethernet or Wi-Fi)

---

## 2. Download Raspberry Pi OS

1. Download **Raspberry Pi Imager** from https://www.raspberrypi.com/software/
2. Install and open it
3. Click **Choose Device** → select your Pi model (Pi 4 or Pi 5)
4. Click **Choose OS** → **Raspberry Pi OS (64-bit)** — this is the Desktop version with LXDE
   - It will say "Debian Bookworm with desktop" underneath
   - Do **NOT** pick the "Lite" version
   - Do **NOT** pick the "Full" version (it includes unnecessary software)
5. Click **Choose Storage** → select your microSD card
6. Click **Next**

### Pre-configure via Imager Settings

When prompted to customise, click **Edit Settings**:

- **Hostname**: `tuix` (or whatever you want)
- **Username and password**: set a username (e.g. `pi`) and a strong password
- **Wi-Fi**: enter your SSID and password
- **Locale**: set your timezone and keyboard layout
- Under the **Services** tab: **Enable SSH** (useful for remote access)

Click **Save**, then **Yes** to apply and write.

---

## 3. First Boot

1. Insert the microSD into the Pi
2. Connect the ROADOM touchscreen via HDMI and USB
3. Connect power
4. The Pi will boot into the desktop setup wizard — follow the prompts
5. Once on the desktop, open a terminal: **Menu → Accessories → LXTerminal**

### Update the system

```bash
sudo apt update && sudo apt upgrade -y
```

---

## 4. Configure the Touchscreen

The ROADOM 7" screen should work out of the box over HDMI. If touch input works via USB immediately, skip to the next section.

### If touch needs calibration

```bash
sudo apt install -y xinput-calibrator
xinput_calibrator
```

Follow the on-screen prompts to tap the crosshairs, then save the calibration output as instructed.

### Set the display resolution (if needed)

Edit the boot config:

```bash
sudo nano /boot/firmware/config.txt
```

Add or modify:

```ini
hdmi_group=2
hdmi_mode=87
hdmi_cvt=1024 600 60 6 0 0 0
hdmi_drive=2
```

Save (`Ctrl+O`, `Enter`, `Ctrl+X`) and reboot:

```bash
sudo reboot
```

---

## 5. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- Select option **1** (default installation)
- After it finishes, load the environment:

```bash
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
cargo --version
```

---

## 6. Install Build Dependencies

TUIX needs a C linker and some system libraries:

```bash
sudo apt install -y build-essential pkg-config git
```

---

## 7. Clone the TUIX Repository

```bash
cd ~/Desktop
git clone https://github.com/YOUR_USERNAME/TUIX-Rust.git
cd TUIX-Rust
```

Replace `YOUR_USERNAME` with the actual GitHub username or repository URL.

---

## 8. Clone Third-Party Dashboards

Run the clone script to install any registered third-party dashboards (e.g. sigye):

```bash
chmod +x x/clone.sh
./x/clone.sh
```

If sigye is registered in `config/dashboards.json`, install its binary:

```bash
cargo install sigye
```

---

## 9. Build and Run TUIX

### First build (takes a few minutes on Pi 4)

```bash
cargo build --release
```

### Run it

```bash
cargo run --release
```

TUIX should now render in the terminal. Use arrow keys or WASD to navigate, Enter to select, Esc to quit.

---

## 10. Auto-Start TUIX on Boot

This will make TUIX launch automatically in a fullscreen terminal when the Pi boots into the desktop.

### Create an autostart entry

```bash
mkdir -p ~/.config/autostart
nano ~/.config/autostart/tuix.desktop
```

Paste the following:

```ini
[Desktop Entry]
Type=Application
Name=TUIX
Comment=Terminal UI Experience
Exec=lxterminal --geometry=200x60 -e bash -c "cd ~/Desktop/TUIX-Rust && cargo run --release; exec bash"
Terminal=false
X-GNOME-Autostart-enabled=true
```

Save and exit (`Ctrl+O`, `Enter`, `Ctrl+X`).

### Make the terminal fullscreen by default

To launch the terminal maximized, edit the LXTerminal config:

```bash
nano ~/.config/lxterminal/lxterminal.conf
```

Under the `[general]` section, add or set:

```ini
geometry_columns=200
geometry_rows=60
```

### Optional: hide the desktop taskbar

If you want a kiosk-like experience where TUIX takes up the whole screen:

```bash
nano ~/.config/lxpanel/LXDE-pi/panels/panel
```

Find the line `autohide=0` and change it to:

```ini
autohide=1
```

### Optional: disable screen blanking

Prevent the screen from going to sleep:

```bash
sudo nano /etc/xdg/lxsession/LXDE-pi/autostart
```

Add these lines at the end:

```
@xset s off
@xset -dpms
@xset s noblank
```

### Reboot to test

```bash
sudo reboot
```

The Pi should boot into the desktop and automatically open TUIX in a terminal window.

---

## 11. Useful Commands

| Action | Command |
|---|---|
| Run TUIX | `cd ~/Desktop/TUIX-Rust && cargo run --release` |
| Rebuild after code changes | `cargo build --release` |
| Update from git | `git pull && cargo build --release` |
| Clone third-party dashboards | `./x/clone.sh` |
| SSH into the Pi | `ssh pi@tuix.local` |
| Check Pi temperature | `vcgencmd measure_temp` |
| Shutdown | `sudo shutdown -h now` |
| Reboot | `sudo reboot` |

---

## 12. Troubleshooting

### Touch not working
- Make sure the USB cable from the touchscreen is connected to the Pi
- Try a different USB port
- Check `dmesg | tail -20` after plugging in for any errors

### Display resolution wrong
- Double-check `config.txt` settings in step 4
- Try `tvservice -s` to see what the Pi detects

### Build fails with out of memory
On a 2GB Pi 4, large Rust builds can run out of RAM. Add swap:

```bash
sudo dphys-swapfile swapoff
sudo nano /etc/dphys-swapfile
```

Set `CONF_SWAPSIZE=2048`, then:

```bash
sudo dphys-swapfile setup
sudo dphys-swapfile swapon
```

### TUIX doesn't auto-start
- Check that `~/.config/autostart/tuix.desktop` exists and has the correct path
- Make sure `cargo` is in your PATH: add `source "$HOME/.cargo/env"` to `~/.bashrc`

### Sigye or third-party dashboard doesn't render
- Make sure the binary is installed: `which sigye`
- If not, install it: `cargo install sigye`
