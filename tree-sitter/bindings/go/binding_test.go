package tree_sitter_rhizome_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_rhizome "github.com/niltonvolpato/rhizome/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_rhizome.Language())
	if language == nil {
		t.Errorf("Error loading Rhizome Language Parser grammar")
	}
}
