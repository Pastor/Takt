// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "TreeSitterBut",
    products: [
        .library(name: "TreeSitterBut", targets: ["TreeSitterBut"]),
    ],
    dependencies: [
        .package(url: "https://github.com/ChimeHQ/SwiftTreeSitter", from: "0.8.0"),
    ],
    targets: [
        .target(
            name: "TreeSitterBut",
            dependencies: [],
            path: ".",
            sources: [
                "src/parser.c",
                // NOTE: if your language has an external scanner, add it here.
            ],
            resources: [
                .copy("queries")
            ],
            publicHeadersPath: "bindings/swift",
            cSettings: [.headerSearchPath("src")]
        ),
        .testTarget(
            name: "TreeSitterButTests",
            dependencies: [
                "SwiftTreeSitter",
                "TreeSitterBut",
            ],
            path: "bindings/swift/TreeSitterButTests"
        )
    ],
    cLanguageStandard: .c11
)
