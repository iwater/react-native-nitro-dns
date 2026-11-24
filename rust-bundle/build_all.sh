#!/bin/bash
# build_all.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Building React Native Rust DNS..."

AUTO_CONFIRM=false
while getopts "y" opt; do
  case $opt in
    y)
      AUTO_CONFIRM=true
      echo "🤖 Auto-confirm enabled (-y)"
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      exit 1
      ;;
  esac
done

echo "🧹 Cleaning previous builds..."
cargo clean

echo "🔧 Building Rust library..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Failed to build Rust library"
    exit 1
fi

echo "✅ Rust library built successfully"

# iOS Build
if [ "$AUTO_CONFIRM" = true ]; then
    REPLY="y"
else
    read -p "Do you want to build iOS libraries? (y/n): " -n 1 -r
    echo
fi
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "📱 Building iOS libraries..."
    mkdir -p target/xcframework
    
    IOS_TARGETS=("x86_64-apple-ios" "aarch64-apple-ios-sim" "aarch64-apple-ios")
    
    for target in "${IOS_TARGETS[@]}"; do
        echo "  Building for $target..."
        cargo build --target "$target" --release
        
        if [ $? -ne 0 ]; then
            echo "❌ Failed to build for $target"
            exit 1
        fi
    done
    
    # ===== Dynamic Framework Build =====
    
    # Create universal simulator dynamic library
    echo "  Creating universal simulator dynamic library..."
    mkdir -p target/universal/ios-sim
    xcrun lipo -create \
        target/x86_64-apple-ios/release/librust_c_dns.dylib \
        target/aarch64-apple-ios-sim/release/librust_c_dns.dylib \
        -output target/universal/ios-sim/librust_c_dns.dylib
    
    # Create device dynamic library directory
    echo "  Preparing device dynamic library..."
    mkdir -p target/universal/ios-device
    cp target/aarch64-apple-ios/release/librust_c_dns.dylib \
       target/universal/ios-device/librust_c_dns.dylib
       
    # Set correct install_name
    echo "  Setting install names..."
    install_name_tool -id @rpath/RNDns.framework/RNDns \
        target/universal/ios-sim/librust_c_dns.dylib
    install_name_tool -id @rpath/RNDns.framework/RNDns \
        target/universal/ios-device/librust_c_dns.dylib
        
    # Create Framework structure
    echo "  Creating Framework structure..."
    FRAMEWORK_SIM="target/universal/ios-sim/RNDns.framework"
    FRAMEWORK_DEVICE="target/universal/ios-device/RNDns.framework"
    
    # Clean old Framework
    rm -rf "$FRAMEWORK_SIM" "$FRAMEWORK_DEVICE"
    
    # Create Simulator Framework
    mkdir -p "$FRAMEWORK_SIM/Headers"
    cp target/universal/ios-sim/librust_c_dns.dylib "$FRAMEWORK_SIM/RNDns"
    cp include/*.h "$FRAMEWORK_SIM/Headers/"
    
    # Create Device Framework
    mkdir -p "$FRAMEWORK_DEVICE/Headers"
    cp target/universal/ios-device/librust_c_dns.dylib "$FRAMEWORK_DEVICE/RNDns"
    cp include/*.h "$FRAMEWORK_DEVICE/Headers/"
    
    # Create Info.plist for Frameworks
    cat > "$FRAMEWORK_SIM/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>RNDns</string>
    <key>CFBundleIdentifier</key>
    <string>com.iwater.RNDns</string>
    <key>CFBundleName</key>
    <string>RNDns</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>MinimumOSVersion</key>
    <string>13.0</string>
</dict>
</plist>
PLIST
    cp "$FRAMEWORK_SIM/Info.plist" "$FRAMEWORK_DEVICE/Info.plist"
    
    # Generate XCFramework
    echo "  Creating XCFramework..."
    rm -rf target/xcframework/RNDns.xcframework
    
    xcodebuild -create-xcframework \
        -framework "$FRAMEWORK_SIM" \
        -framework "$FRAMEWORK_DEVICE" \
        -output target/xcframework/RNDns.xcframework
    
    if [ $? -eq 0 ]; then
        echo "✅ iOS XCFramework built successfully!"
    else
        echo "❌ Failed to create XCFramework"
        exit 1
    fi
fi

# Android Build
if [ "$AUTO_CONFIRM" = true ]; then
    REPLY="y"
else
    read -p "Do you want to build Android libraries? (y/n): " -n 1 -r
    echo
fi
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🤖 Building Android libraries..."
    
    if [ -z "$ANDROID_NDK_HOME" ]; then
        # Default fallback
        ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/27.1.12297006"
    fi
    
    ANDROID_TARGETS=(
        "aarch64-linux-android"
        "armv7-linux-androideabi" 
        "i686-linux-android"
        "x86_64-linux-android"
    )
    
    for target in "${ANDROID_TARGETS[@]}"; do
        echo "  Building for $target..."
        RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" cargo ndk --target "$target" --platform 21 build --release
        
        if [ $? -ne 0 ]; then
            echo "❌ Failed to build for $target"
            exit 1
        fi
    done
    
    echo "✅ Android libraries built successfully!"
fi

echo "🧪 Running Rust tests..."
cargo test -- --test-threads=1 --nocapture

echo ""
echo "🎉 Build completed for RNDns!"
