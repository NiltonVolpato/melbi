import XCTest
import SwiftTreeSitter
import TreeSitterRhizome

final class TreeSitterRhizomeTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_rhizome())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Rhizome Language Parser grammar")
    }
}
