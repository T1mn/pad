// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "PADDesktop",
    platforms: [
        .macOS(.v13),
    ],
    products: [
        .executable(name: "PADDesktop", targets: ["PADDesktopApp"]),
    ],
    targets: [
        .executableTarget(
            name: "PADDesktopApp",
            path: "Sources/PADDesktopApp"
        ),
    ]
)
