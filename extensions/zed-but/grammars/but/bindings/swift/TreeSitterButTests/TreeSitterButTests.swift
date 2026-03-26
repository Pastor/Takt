import XCTest
import SwiftTreeSitter
import TreeSitterBut

final class TreeSitterButTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_but())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading But grammar")
    }
}
