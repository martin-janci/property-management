#!/usr/bin/env bash
# setup-android-keystore.sh
#
# Epic 85 - Story 85.2: Android Release Build Configuration
#
# Generates the release keystore for the PPT Management Android app
# (three.two.bit.ppt.management) and prints the EAS secret upload command.
#
# Usage:
#   ./scripts/setup-android-keystore.sh
#
# The generated keystore is placed at:
#   frontend/apps/mobile/android-keystore/ppt-management-release.keystore
#
# SECURITY NOTES:
#   - Never commit the generated .keystore file. It is listed in .gitignore.
#   - Store the keystore and passwords in a secure vault (e.g. 1Password, Bitwarden).
#   - If the keystore is lost and the app is published, you cannot update the app
#     on the Play Store. Back it up securely before using it in production.
#   - For EAS Remote builds the keystore is uploaded to Expo's encrypted store via:
#       eas credentials --platform android
#     or by setting the ANDROID_KEYSTORE, ANDROID_KEY_ALIAS, etc. EAS secrets.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
KEYSTORE_DIR="${REPO_ROOT}/frontend/apps/mobile/android-keystore"
KEYSTORE_FILE="${KEYSTORE_DIR}/ppt-management-release.keystore"

KEY_ALIAS="ppt-management"
KEYSTORE_VALIDITY_DAYS=10950  # 30 years
DISTINGUISHED_NAME="CN=PPT Management, OU=Mobile, O=32bit s.r.o., L=Bratislava, ST=Slovakia, C=SK"

echo "=== PPT Management Android Release Keystore Setup ==="
echo ""

# Check for keytool
if ! command -v keytool &>/dev/null; then
  echo "ERROR: keytool not found. Install a JDK (e.g. 'apt install default-jdk')." >&2
  exit 1
fi

# Create output directory
mkdir -p "${KEYSTORE_DIR}"

# Check if keystore already exists
if [[ -f "${KEYSTORE_FILE}" ]]; then
  echo "WARNING: Keystore already exists at:"
  echo "  ${KEYSTORE_FILE}"
  echo ""
  echo "To regenerate, delete the file first. Reusing an existing keystore is"
  echo "STRONGLY PREFERRED once the app is published to the Play Store."
  echo ""
  read -r -p "Print keystore certificate info instead? [y/N]: " SHOW_INFO
  if [[ "${SHOW_INFO,,}" == "y" ]]; then
    keytool -list -v \
      -keystore "${KEYSTORE_FILE}" \
      -alias "${KEY_ALIAS}"
  fi
  exit 0
fi

echo "Keystore path : ${KEYSTORE_FILE}"
echo "Key alias     : ${KEY_ALIAS}"
echo "Validity      : ${KEYSTORE_VALIDITY_DAYS} days (~30 years)"
echo ""
echo "You will be prompted for:"
echo "  - Keystore password  (store securely — required for every release build)"
echo "  - Key password       (can be same as keystore password)"
echo ""

keytool -genkeypair \
  -keystore "${KEYSTORE_FILE}" \
  -alias "${KEY_ALIAS}" \
  -keyalg RSA \
  -keysize 4096 \
  -validity "${KEYSTORE_VALIDITY_DAYS}" \
  -dname "${DISTINGUISHED_NAME}" \
  -storetype PKCS12

echo ""
echo "=== Keystore generated successfully ==="
echo ""
echo "NEXT STEPS"
echo "----------"
echo ""
echo "1. Print the SHA-256 certificate fingerprint (needed for App Links / assetlinks.json):"
echo ""
echo "   keytool -list -v -keystore ${KEYSTORE_FILE} -alias ${KEY_ALIAS}"
echo ""
echo "2. Upload to EAS (recommended for cloud builds):"
echo ""
echo "   eas credentials --platform android"
echo "   # Follow the interactive prompts to upload the keystore."
echo ""
echo "   OR set EAS secrets manually:"
echo ""
echo "   eas secret:create --scope project --name ANDROID_KEYSTORE \\"
echo "     --value \"\$(base64 -w0 ${KEYSTORE_FILE})\""
echo "   eas secret:create --scope project --name ANDROID_KEY_ALIAS \\"
echo "     --value '${KEY_ALIAS}'"
echo "   eas secret:create --scope project --name ANDROID_KEYSTORE_PASSWORD \\"
echo "     --value 'YOUR_KEYSTORE_PASSWORD'"
echo "   eas secret:create --scope project --name ANDROID_KEY_PASSWORD \\"
echo "     --value 'YOUR_KEY_PASSWORD'"
echo ""
echo "3. Back up the keystore file and passwords to a secure vault before using"
echo "   this keystore in a Play Store production release."
echo ""
echo "4. Update /.well-known/assetlinks.json on your server with the SHA-256"
echo "   fingerprint to enable Android App Links."
echo ""
echo "IMPORTANT: ${KEYSTORE_FILE} is gitignored. Do NOT commit it."
