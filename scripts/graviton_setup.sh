#!/bin/bash
#
# Setup Graviton instance: Install dependencies and compile ASBB
#
# Lab Notebook: Entry 021
# Usage: ./scripts/graviton_setup.sh <public-ip>
#

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <public-ip>"
    exit 1
fi

PUBLIC_IP=$1
KEY_NAME="asbb-graviton-key"
SSH_KEY=~/.ssh/${KEY_NAME}.pem

echo "=== Graviton Instance Setup ==="
echo "Public IP: $PUBLIC_IP"
echo

# Create setup script to run on instance
cat > /tmp/graviton_setup_remote.sh <<'EOF'
#!/bin/bash
set -e

echo "=== Remote Setup Started ==="
echo

# Update system
echo "1. Updating system packages..."
sudo yum update -y
echo "✅ System updated"
echo

# Install development tools
echo "2. Installing development tools..."
sudo yum install -y gcc git cmake
echo "✅ Development tools installed"
echo

# Install Rust
echo "3. Installing Rust toolchain..."
if [ ! -f ~/.cargo/bin/rustc ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    echo "✅ Rust installed"
else
    source ~/.cargo/env
    echo "✅ Rust already installed"
fi
echo

# Verify Rust installation
echo "4. Verifying Rust installation..."
rustc --version
cargo --version
echo "✅ Rust verified"
echo

# Clone ASBB repository
echo "5. Cloning ASBB repository..."
if [ -d ~/asbb ]; then
    echo "Repository already exists, pulling latest..."
    cd ~/asbb
    git pull
else
    cd ~
    # Use HTTPS to avoid SSH key issues
    git clone https://github.com/scotthandley/apple-silicon-bio-bench.git asbb || \
    git clone https://github.com/YOUR_USERNAME/apple-silicon-bio-bench.git asbb || \
    echo "⚠️  Repository clone failed - will need to transfer code manually"
fi
echo

# If git clone failed, we'll transfer code via SCP later
if [ ! -d ~/asbb ]; then
    echo "Creating asbb directory for code transfer..."
    mkdir -p ~/asbb
fi

echo "=== Remote Setup Complete ==="
EOF

# Transfer and execute setup script
echo "Transferring setup script to instance..."
scp -i "$SSH_KEY" -o StrictHostKeyChecking=no /tmp/graviton_setup_remote.sh ec2-user@${PUBLIC_IP}:~/setup.sh
echo

echo "Executing setup script on instance (this may take 5-10 minutes)..."
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "chmod +x ~/setup.sh && ~/setup.sh"
echo

# Check if repository was cloned successfully
REPO_EXISTS=$(ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "[ -f ~/asbb/Cargo.toml ] && echo 'yes' || echo 'no'")

if [ "$REPO_EXISTS" == "no" ]; then
    echo "Repository not found on instance, transferring code via SCP..."

    # Create tarball of local code
    echo "Creating code tarball..."
    tar -czf /tmp/asbb-code.tar.gz \
        --exclude=target \
        --exclude=.git \
        --exclude=results \
        --exclude='*.csv' \
        --exclude='*.txt' \
        -C /Users/scotthandley/Code \
        apple-silicon-bio-bench

    # Transfer tarball
    echo "Transferring code to instance..."
    scp -i "$SSH_KEY" -o StrictHostKeyChecking=no /tmp/asbb-code.tar.gz ec2-user@${PUBLIC_IP}:~/

    # Extract on instance
    echo "Extracting code on instance..."
    ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "tar -xzf asbb-code.tar.gz && mv apple-silicon-bio-bench asbb && rm asbb-code.tar.gz"

    # Cleanup
    rm /tmp/asbb-code.tar.gz
    echo "✅ Code transferred successfully"
fi

# Compile pilot binary
echo
echo "Compiling Graviton pilot binary..."
ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "source ~/.cargo/env && cd ~/asbb/crates/asbb-cli && cargo build --release --bin asbb-pilot-graviton"
echo

# Verify compilation
echo "Verifying compilation..."
BINARY_EXISTS=$(ssh -i "$SSH_KEY" -o StrictHostKeyChecking=no ec2-user@${PUBLIC_IP} "[ -f ~/asbb/target/release/asbb-pilot-graviton ] && echo 'yes' || echo 'no'")

if [ "$BINARY_EXISTS" == "yes" ]; then
    echo "✅ Binary compiled successfully"
else
    echo "❌ Binary compilation failed"
    exit 1
fi

echo
echo "=== Setup Complete ==="
echo "Next step: ./scripts/graviton_run.sh $PUBLIC_IP"
