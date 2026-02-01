API_SOCKET="/tmp/firecracker.socket"

# Remove API unix socket
sudo rm -f $API_SOCKET

# Read this dynamically from 
firecracker=$(pwd)/firecracker

# Run firecracker
sudo ./firecracker --api-sock "${API_SOCKET}" --enable-pci
