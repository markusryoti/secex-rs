API_SOCKET="/tmp/firecracker.socket"

# Remove API unix socket
sudo rm -f $API_SOCKET

firecracker="/home/markusryoti/Downloads/firecracker-v1.14.1-x86_64/release-v1.14.1-x86_64"

# Run firecracker
sudo ./firecracker --api-sock "${API_SOCKET}" --enable-pci