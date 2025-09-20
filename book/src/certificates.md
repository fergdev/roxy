# Roxy — Certificate Authority (CA) and installing the CA certificate

Roxy creates a local Certificate Authority (CA) the first time it runs. That CA is used to generate short-lived leaf certificates on the fly so Roxy can intercept and inspect HTTPS traffic. Browsers and OSes do not trust this CA by default, so to avoid TLS warnings you’ll want to install the Roxy CA cert into the appropriate trust store on the machine or device you’re testing.

Below you’ll find:

- What files Roxy writes
- Short descriptions of each file and recommended permissions
- How to use the certs from command line tools (curl/wget) while developing
- Platform-specific installation instructions (macOS, Linux, Windows, iOS, Android, Java, browsers)
- Security notes and quick troubleshooting tips

## Files created by Roxy

When Roxy generates the CA it stores a small set of files in the config directory (e.g. <b>~/.roxy</b> by default). Example listing:

```bash
.rw-r--r--  1.2k  20 Sep 07:23  roxy-ca-cert.cer
.rw-r--r--  1.2k  20 Sep 07:24  roxy-ca-cert.p12
.rw-r--r--  1.2k  20 Sep 07:23  roxy-ca-cert.pem
.rw-r--r--  2.9k  20 Sep 07:23  roxy-ca.cer
.rw-r--r--  2.7k  20 Sep 07:24  roxy-ca.p12
.rw-r--r--  2.9k  20 Sep 07:23  roxy-ca.pem
```

What each file is for

| Filename | Use |
| -------------- | --------------- |
| roxy-ca.pem | The CA certificate and private key in PEM format (combined). Keep this private — it contains the private key.|
| roxy-ca-cert.pem | The CA certificate only (PEM). Use this to import into most OS and app trust stores. |
| roxy-ca-cert.p12 | The CA certificate in PKCS#12 format (contains cert and private key). Useful for systems that expect a .p12 bundle. Protect this file. |
| roxy-ca-cert.cer | A certificate file with a .cer extension (PEM-encoded). Some devices expect .cer when installing. |
| roxy-ca.cer<br/>roxy-ca.p12<br/>roxy-ca.pem | Alternate names / copies that some tooling expects; the .cer is functionally the cert, .p12 is PKCS#12 bundle, .pem is PEM-encoded. Roxy writes multiple file extensions for maximum compatibility. |

Permissions recommendation: make the private-key-containing files readable only by you:

```bash
chmod 600 ~/.roxy/roxy-ca.pem
chmod 600 ~/.roxy/roxy-ca-cert.p12
```

## Why Roxy generates a CA and why it’s local-only

Roxy generates a unique CA on first run so that your intercepted traffic stays private to your machine. The CA private key is never shared between installations — this prevents another machine’s Roxy instance from being able to decrypt your traffic. If the CA private key is ever compromised, you should remove the CA from trust stores and regenerate a new CA.

## Quick CLI examples (using Roxy as proxy)

If you just want to test a single HTTPS request through Roxy without installing the CA globally, pipe the roxy-ca-cert.pem to your tools:

### curl

```bash
curl --proxy 127.0.0.1:8080 --cacert ~/.roxy/roxy-ca-cert.pem <https://example.com/>
```

### wget

```bash
wget -e https_proxy=127.0.0.1:8080 --ca-certificate ~/.roxy/roxy-ca-cert.pem <https://example.com/>
```

Replace 127.0.0.1:8080 with the host:port your Roxy instance listens on.

## Platform installation guide

Note: exact UI steps vary by OS version. When possible prefer importing the PEM (roxy-ca-cert.pem) into the system trust store rather than a per-user store, especially for browsers and system services.

### macOS (System-wide)

Install and trust the certificate in the macOS system keychain:

```bash
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/.roxy/roxy-ca-cert.pem
```

You can also double-click roxy-ca-cert.pem to open Keychain Access, add it to the System keychain, then open the certificate and set Trust → When using this certificate → Always Trust.

### Ubuntu / Debian

For command-line tools and most system services:

```bash
sudo cp ~/.roxy/roxy-ca-cert.pem /usr/local/share/ca-certificates/roxy-ca.crt
sudo update-ca-certificates
```

### Fedora / RHEL / CentOS

```bash
sudo cp ~/.roxy/roxy-ca-cert.pem /etc/pki/ca-trust/source/anchors/
sudo update-ca-trust extract
```

### Arch Linux

```bash
sudo trust anchor --store ~/.roxy/roxy-ca-cert.pem
```

(or use ca-certificates package instructions for your distro variant).

### Mozilla Firefox (Linux / macOS / Windows)

Firefox maintains its own certificate store. Import the roxy-ca-cert.pem via:
Preferences → Privacy & Security → View Certificates → Authorities → Import… and enable trusting for websites.

Alternatively, use certutil (from nss-tools / libnss3-tools) to import into a profile programmatically.

### Google Chrome on Linux

If Chrome is built to use the system store, installing into the system CA (see Debian/Ubuntu steps) is enough. If not, you may need to import into the NSS DB used by Chrome (see certutil usage for your distro).

### Windows

Import the .cer into the Windows Trusted Root Certification Authorities store:
 • Double-click roxy-ca-cert.cer → Install Certificate → Local Machine → Place in Trusted Root Certification Authorities.
 • Or use certutil (run as admin):

```powershell
certutil -addstore -f "ROOT" C:\path\to\roxy-ca-cert.cer
```

If you need a PKCS#12 bundle for some Windows tools or browsers, use roxy-ca-cert.p12.

### iOS (real devices)

On recent iOS versions you must both install and enable full trust:

 1. Copy roxy-ca-cert.cer to the device (e.g., send via email or host on a local webserver).
 2. Open the file on the device; iOS will add the profile in Settings → General → VPN & Device Management (or Profiles).
 3. After installing, go to Settings → General → About → Certificate Trust Settings and enable full trust for the installed Roxy certificate.

### iOS Simulator

 1. Ensure macOS is configured to proxy the simulator network through Roxy.
 2. In the simulator, open Safari and visit a URL that serves roxy-ca-cert.cer (you can host on localhost and use macOS forwarding).
 3. Install the cert in the simulator settings and enable full trust (see real device steps).

### Android (device)

Android behaviour differs between versions:
 • User-installed CA certs are not trusted by all apps on Android 7+ by default (apps can opt out of user CAs). For system-wide trust you must install the cert to the system store — this requires root or building the cert into the system image.
 • For development and testing on devices:
 • Convert the PEM to DER if needed:

```bash
openssl x509 -in ~/.roxy/roxy-ca-cert.pem -outform DER -out roxy-ca-cert.der
```

- Copy roxy-ca-cert.cer / .der to the device and install via Settings → Security → Install from storage (UI varies).
- For emulators you can push the cert into the emulator system store or use the simulator-device instructions.

### Java (JVM)

To add the Roxy CA to the JVM cacerts store (system-wide JDK):

```bash
sudo keytool -importcert -trustcacerts -alias roxy -file ~/.roxy/roxy-ca-cert.pem \
  -keystore $JAVA_HOME/lib/security/cacerts -storepass changeit
```

(Replace $JAVA_HOME with your JDK path and adjust -storepass if your cacerts password differs.)

### Creating a .p12 or .cer from PEM (if needed)

If you only have a PEM and need a PKCS#12 bundle (for Windows/macOS imports):

```bash
openssl pkcs12 -export -out roxy-ca-cert.p12 -inkey roxy-ca.pem -in roxy-ca-cert.pem -passout pass:changeit
```

To convert PEM to DER (.der/.cer) for Android:

```bash
openssl x509 -in roxy-ca-cert.pem -outform DER -out roxy-ca-cert.der
```

### Verifying the certificate fingerprint

Always verify the certificate fingerprint before trusting/distributing it:

```bash
openssl x509 -in ~/.roxy/roxy-ca-cert.pem -noout -fingerprint -sha256
```

Example output: SHA256 Fingerprint=AA:BB:CC:...:ZZ

Publish the fingerprint (or display it in your UI) so users can confirm they installed the correct CA.

Security recommendations

- Never distribute roxy-ca.pem or roxy-ca-cert.p12 (both contain the private key) unless absolutely necessary and only over a secure channel.
- Keep private-key files (roxy-ca.pem, *.p12) permissioned to owner-only: chmod 600.
- If the machine is shared or exposed, revoke and regenerate the CA and remove it from any systems it was installed into.
- Log and audit where you installed the CA so you can revoke/trust changes consistently.
- Prefer per-development-machine CA rather than a shared global CA for testing.

### Troubleshooting

- Still seeing TLS warnings: confirm the cert was installed in the system trust store (not just the browser’s temporary profile). Check the cert fingerprint.
- Browser still rejects after installing system CA: some browsers (Firefox on many platforms) use their own certificate store — import into Firefox separately.
- Android app refuses: modern Android apps may opt out of user CAs — either install CA into system store (requires root) or configure the app/network stack to trust the certificate for development.
- Java apps fail: ensure the CA is in the JVM cacerts used by that runtime. Different JDKs/containers may have separate cacerts files.
- Proxy not being used: ensure your client is sending traffic via Roxy (check host/port, and that the proxy accepts TLS CONNECT or is configured as a transparent proxy).

### Example checklist for onboarding a new dev machine

 1. Start Roxy once to allow it to generate ~/.roxy/* files.
 2. Verify fingerprint:

```bash
openssl x509 -in ~/.roxy/roxy-ca-cert.pem -noout -fingerprint -sha256
```

 3. Install roxy-ca-cert.pem into your OS/browser as required (use steps above).
 4. Confirm with curl:

```bash
curl --proxy 127.0.0.1:8080 --cacert ~/.roxy/roxy-ca-cert.pem <https://example.com/> -v
```

If TLS succeeds without cert warnings, you’re good.
