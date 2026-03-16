// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "SwiftLib",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "SwiftLib", type: .static, targets: ["SwiftLib"])
    ],
    dependencies: [
        .package(url: "https://github.com/Brendonovich/swift-rs", from: "1.0.7")
    ],
    targets: [
        .target(
            name: "SwiftLib",
            dependencies: [.product(name: "SwiftRs", package: "swift-rs")],
            path: "Sources/SwiftLib"
        )
    ]
)
