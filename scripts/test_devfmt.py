#!/usr/bin/env python3
"""Tests for devfmt.py. Run with `python3 scripts/test_devfmt.py`."""

import unittest

from devfmt import (
    LANGUAGES,
    MARKDOWN,
    ROOT,
    UNSUPPORTED,
    classify,
    paragraph_hunks,
    parse_diff_touched_lines,
    reflow_markdown,
    reflow_paragraphs,
    reflow_source,
    wanted,
)

RUST = classify("x.rs")


def rust(text, width, touched=None):
    return reflow_source(text, RUST, width, touched)


def lines_of(text):
    return text.splitlines()


class Languages(unittest.TestCase):
    def test_a_known_language_is_reflowed(self):
        # Panoply exempts `src` and the other source trees, so these paths are elsewhere.
        self.assertEqual(classify("lib/a.rs").name, "Rust")
        self.assertEqual(classify("docs/a.md"), MARKDOWN)

    def test_source_in_an_unknown_language_is_refused_rather_than_skipped(self):
        for path in ["lib/a.cpp", "include/a.h", "lib/a.HPP", "tools/a.py"]:
            self.assertEqual(classify(path), UNSUPPORTED, f"{path} would have been skipped silently")
            self.assertTrue(wanted(path), f"{path} would not reach the error")

    def test_an_exempt_path_is_skipped_whatever_its_language(self):
        # Relative paths resolve against the working directory, so these are absolute.
        for name in ["devfmt.py", "verify.sh", "notes.md"]:
            path = ROOT / "scripts" / name
            self.assertIsNone(classify(path), name)
            self.assertFalse(wanted(path), name)
        self.assertEqual(classify(ROOT / "docs" / "a.md"), MARKDOWN)

    def test_a_file_with_no_prose_is_skipped(self):
        for path in ["a.png", "Cargo.lock", "a.json", "Makefile"]:
            self.assertIsNone(classify(path), path)
            self.assertFalse(wanted(path), path)

    def test_every_marker_that_begins_with_another_comes_first(self):
        # `//` before `///` would read every doc comment as a plain comment whose text starts `/`.
        for lang in LANGUAGES:
            for i, earlier in enumerate(lang.markers):
                for later in lang.markers[i + 1 :]:
                    self.assertFalse(
                        later.startswith(earlier), f"{lang.name}: {later!r} is shadowed by {earlier!r}"
                    )


class Reflow(unittest.TestCase):
    def assertFits(self, out, width):
        for line in lines_of(out):
            self.assertLessEqual(len(line), width, f"line too long: {line!r}")

    def test_a_short_paragraph_is_unchanged(self):
        text = "/// One short line.\n"
        self.assertEqual(rust(text, 100), text)

    def test_a_long_paragraph_wraps_at_the_width(self):
        text = "/// aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
        out = rust(text, 40)
        self.assertFits(out, 40)
        self.assertEqual(
            out.replace("/// ", "").replace("\n", " ").strip(),
            "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq",
        )

    def test_already_correctly_wrapped_text_is_idempotent(self):
        text = "/// aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj\n/// kkkk llll mmmm nnnn oooo pppp qqqq\n"
        once = rust(text, 40)
        self.assertNotEqual(once, text, "the input's first line is over width, so this should have reflowed")
        self.assertEqual(rust(once, 40), once)

    def test_a_paragraph_wrapped_short_of_the_limit_is_repacked(self):
        # Nothing here violates the width. The paragraph is ragged, which is what an edit that
        # shortens a line leaves behind and precisely what a width test cannot see. What keeps this
        # off prose nobody edited is the diff scope, not the width — the two tests below.
        self.assertEqual(rust("/// aaaa bbbb\n/// cccc dddd\n", 100), "/// aaaa bbbb cccc dddd\n")

    def test_a_ragged_paragraph_outside_the_diff_is_left_alone(self):
        # With the width no longer gating a rewrap, scope is the only thing between a routine run
        # and every paragraph in the file, so this is the guard rail that matters.
        text = "/// aaaa bbbb\n/// cccc dddd\n"
        self.assertEqual(rust(text, 100, set()), text)

    def test_a_ragged_paragraph_inside_the_diff_is_repacked(self):
        self.assertEqual(rust("/// aaaa bbbb\n/// cccc dddd\n", 100, {1}), "/// aaaa bbbb cccc dddd\n")

    def test_repacking_a_packed_paragraph_changes_nothing(self):
        # A sweep over an already-packed tree has to be a no-op, or a migration could never finish.
        packed = rust("/// aaaa bbbb\n/// cccc dddd\n", 100)
        self.assertEqual(rust(packed, 100), packed)

    def test_a_blank_doc_comment_line_separates_paragraphs(self):
        text = "/// First paragraph.\n///\n/// Second paragraph.\n"
        self.assertEqual(rust(text, 100), text)

    def test_fenced_code_blocks_are_untouched_however_long(self):
        text = "/// ```rust\n/// let x = some_very_long_identifier_that_would_never_fit_on_an_eighty_column_line;\n/// ```\n"
        self.assertEqual(rust(text, 40), text)

    def test_a_doc_comment_inside_a_doc_example_keeps_its_marker(self):
        text = "/// ```rust\n/// /// A resource.\n/// struct Known;\n/// ```\n"
        self.assertEqual(rust(text, 100), text)

    def test_table_rows_are_untouched_however_long(self):
        text = "/// | one very long column header | another very long column header that overflows |\n"
        self.assertEqual(rust(text, 40), text)

    def test_a_list_item_wraps_with_a_hanging_indent(self):
        out = rust("/// - one two three four five six seven eight nine ten eleven twelve thirteen\n", 40)
        out_lines = lines_of(out)
        self.assertTrue(out_lines[0].startswith("/// - one"))
        self.assertTrue(out_lines[1].startswith("///   "), f"continuation not hung: {out_lines[1]!r}")
        self.assertFits(out, 40)

    def test_a_multiline_list_item_that_fits_on_one_line_is_joined(self):
        self.assertEqual(
            rust("/// - one two three four five six seven\n///   eight nine ten\n", 100),
            "/// - one two three four five six seven eight nine ten\n",
        )

    def test_a_multiline_list_item_that_violates_the_width_is_merged_and_rewrapped(self):
        text = "/// - one two three four five six seven eight nine ten eleven twelve thirteen\n///   fourteen\n"
        out = rust(text, 40)
        out_lines = lines_of(out)
        self.assertNotEqual(out, text, "an over-width item must actually reflow")
        self.assertFits(out, 40)
        self.assertTrue(out_lines[0].startswith("/// - one"))
        for line in out_lines[1:]:
            self.assertTrue(line.startswith("///   "), f"continuation not hung: {line!r}")
        words = out.replace("/// - ", "").replace("///   ", " ").replace("\n", " ")
        self.assertEqual(
            words.split(),
            "one two three four five six seven eight nine ten eleven twelve thirteen fourteen".split(),
        )

    def test_different_indentation_never_merges_across_scopes(self):
        text = "    /// inner one\nfn f() {\n        /// deeper one\n    }\n"
        self.assertEqual(rust(text, 100), text)

    def test_an_indented_paragraph_keeps_its_indent(self):
        # A continuation paragraph under a list item. Dedenting it to column 0 takes it out of the
        # item it belongs to, which is a different document.
        out = reflow_paragraphs("  aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk\n", 30, "", 1, None)
        for line in lines_of(out):
            self.assertTrue(line.startswith("  "), f"lost its indent: {line!r}")
        self.assertFits(out, 30)

    def test_a_nested_list_item_keeps_its_nesting(self):
        # Promoting this to column 0 flattens a two-level list into one.
        out_lines = lines_of(reflow_paragraphs("  - aaaa bbbb cccc dddd eeee ffff gggg hhhh\n", 30, "", 1, None))
        self.assertGreater(len(out_lines), 1, "nothing rewrapped, so nothing was tested")
        self.assertTrue(out_lines[0].startswith("  - "), f"promoted: {out_lines[0]!r}")
        for line in out_lines[1:]:
            self.assertTrue(line.startswith("    "), f"continuation not hung: {line!r}")

    def test_a_nested_list_item_in_a_doc_comment_keeps_its_nesting(self):
        out_lines = lines_of(rust("///   - aaaa bbbb cccc dddd eeee ffff gggg hhhh\n", 30))
        self.assertGreater(len(out_lines), 1, "nothing rewrapped, so nothing was tested")
        self.assertTrue(out_lines[0].startswith("///   - "), f"promoted: {out_lines[0]!r}")
        for line in out_lines[1:]:
            self.assertTrue(line.startswith("///     "), f"continuation not hung: {line!r}")

    def test_a_divider_is_not_absorbed_into_the_paragraph_below_it(self):
        out = rust("// -------------------\n// aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii\n", 30)
        self.assertTrue(out.startswith("// -------------------\n"), f"the divider grew a word: {out!r}")

    def test_a_backtick_span_containing_a_space_is_never_split(self):
        # Splitting `12. ` leaves a fragment that a later run over the same file reads as a list
        # marker, so the damage compounds.
        out = rust("/// so `- ` and `12. ` each hang their continuation lines under their own width\n", 40)
        self.assertTrue(any("`12. `" in l for l in lines_of(out)), f"the span was split: {out!r}")

    def test_an_unpaired_backtick_falls_back_to_plain_words(self):
        # One stray backtick would otherwise glue the rest of the paragraph into a single unit.
        out = rust("/// a stray ` tick aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk\n", 30)
        self.assertFits(out, 30)

    def test_plain_line_comments_reflow_the_same_as_doc_comments(self):
        out = rust("    // aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo\n", 40)
        self.assertFits(out, 40)

    def test_code_is_never_touched(self):
        text = "fn f() {\n    let x = 1;\n}\n"
        self.assertEqual(rust(text, 20), text)

    def test_a_link_definition_is_left_alone(self):
        text = (
            "/// See [the crate][crate-link] for a very long description that would otherwise wrap here.\n"
            "///\n"
            "/// [crate-link]: https://example.com/some/extremely/long/path/that/must/not/be/broken/apart\n"
        )
        self.assertIn(
            "[crate-link]: https://example.com/some/extremely/long/path/that/must/not/be/broken/apart\n",
            rust(text, 60),
        )

    def test_markdown_files_reflow_without_a_comment_marker(self):
        text = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
        self.assertFits(reflow_paragraphs(text, 40, "", 1, None), 40)

    def test_a_blockquote_is_held_verbatim(self):
        # Refilled, its `>` markers land mid-line as words and the quote becomes one ragged line.
        text = "> one two three four five six seven\n> eight nine ten\n"
        self.assertEqual(reflow_paragraphs(text, 30, "", 1, None), text)

    def test_markdown_headings_and_blank_lines_pass_through(self):
        text = "# Title\n\nSome text.\n"
        self.assertEqual(reflow_paragraphs(text, 100, "", 1, None), text)

    def test_frontmatter_is_held_verbatim(self):
        text = (
            "---\nname: comment-grooming\ndescription: Sweep comments and doc comments for house-style "
            "violations and duplicated explanations, using recon subagents.\n---\n\n# Heading\n"
        )
        self.assertEqual(reflow_markdown(text, 100, None), text)

    def test_the_body_after_frontmatter_still_reflows(self):
        text = "---\nname: x\n---\n\naaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
        out = reflow_markdown(text, 40, None)
        self.assertTrue(out.startswith("---\nname: x\n---\n\n"))
        self.assertTrue(all(len(l) <= 40 for l in lines_of(out)[4:]))

    def test_frontmatter_does_not_shift_the_body_line_numbers(self):
        text = "---\nname: x\n---\n\naaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
        # The paragraph is on line 5, not line 1 of a body counted from scratch.
        self.assertNotEqual(reflow_markdown(text, 40, {5}), text)
        self.assertEqual(reflow_markdown(text, 40, {2}), text)

    def test_an_unterminated_leading_marker_is_not_frontmatter(self):
        text = "---\naaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
        out = reflow_markdown(text, 40, None)
        self.assertNotEqual(out, text)
        self.assertFits(out, 40)

    def test_a_missing_trailing_newline_gains_one_in_the_engine(self):
        # reflow_file restores the file's own ending; the engine always ends a line.
        self.assertEqual(rust("/// short", 100), "/// short\n")


class DiffScope(unittest.TestCase):
    def test_a_violation_outside_the_touched_lines_is_left_alone(self):
        text = "/// aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
        self.assertEqual(rust(text, 40, set()), text)  # nothing changed in this file

    def test_a_violation_inside_the_touched_lines_still_reflows(self):
        text = "/// aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
        out = rust(text, 40, {1})
        self.assertNotEqual(out, text)
        self.assertTrue(all(len(l) <= 40 for l in lines_of(out)))

    def test_only_the_touched_paragraph_of_several_reflows(self):
        text = (
            "/// aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll mmmm nnnn oooo pppp qqqq\n"
            "///\n"
            "/// AAAA BBBB CCCC DDDD EEEE FFFF GGGG HHHH IIII JJJJ KKKK LLLL MMMM NNNN OOOO PPPP QQQQ\n"
        )
        # Line 3 is the second paragraph; line 1 is the first. Only line 3 is "changed".
        out_lines = lines_of(rust(text, 40, {3}))
        self.assertGreater(len(out_lines[0]), 40, "untouched paragraph must not have reflowed")
        self.assertTrue(all(len(l) <= 40 for l in out_lines[2:]))

    def test_parses_a_simple_addition_hunk(self):
        diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -10,2 +10,3 @@ fn f() {\n"
        self.assertEqual(parse_diff_touched_lines(diff), {10, 11, 12})

    def test_parses_a_single_line_hunk_with_no_count(self):
        # git omits the count when it is 1: `@@ -5 +5 @@` means one line.
        self.assertEqual(parse_diff_touched_lines("@@ -5 +5 @@\n"), {5})

    def test_parses_a_pure_deletion_hunk_as_its_neighbors(self):
        self.assertEqual(parse_diff_touched_lines("@@ -8,2 +7,0 @@\n"), {6, 7})

    def test_ignores_non_hunk_lines(self):
        self.assertEqual(parse_diff_touched_lines("diff --git a/f b/f\nindex 111..222 100644\n--- a/f\n+++ b/f\n"), set())


class Preview(unittest.TestCase):
    def test_an_unchanged_text_has_no_hunks(self):
        text = "keep me\n\naaaa bbbb\n"
        self.assertEqual(paragraph_hunks(text, text), [])

    def test_a_hunk_addresses_the_lines_that_moved(self):
        old = "keep me\n\naaaa bbbb\ncccc\n\nkeep me too\n"
        new = "keep me\n\naaaa bbbb cccc\n\nkeep me too\n"
        hunks = paragraph_hunks(old, new)
        self.assertEqual(len(hunks), 1)
        self.assertEqual((hunks[0].old_start, hunks[0].old_count), (2, 4))
        self.assertEqual((hunks[0].new_start, hunks[0].new_count), (2, 3))
        self.assertEqual(hunks[0].body, [" ", "-aaaa bbbb", "-cccc", "+aaaa bbbb cccc", " "])

    def test_a_hunk_that_only_removes_lines_keeps_its_context(self):
        hunks = paragraph_hunks("aaaa bbbb cccc\ndddd\n", "aaaa bbbb cccc\n")
        self.assertEqual(len(hunks), 1)
        self.assertEqual(hunks[0].body, [" aaaa bbbb cccc", "-dddd"])
        self.assertEqual((hunks[0].old_count, hunks[0].new_count), (2, 1))

    def test_unchanged_lines_between_two_changes_stay_out_of_the_diff(self):
        # A doc comment, the code under it and a second comment share one blank-delimited segment.
        # A change at each end must not carry the untouched code between them into a hunk.
        old = "aaaa bbbb\ncccc\nfn f() {}\nlet x = 1;\nlet y = 2;\ndddd eeee\nffff\n"
        new = "aaaa bbbb cccc\nfn f() {}\nlet x = 1;\nlet y = 2;\ndddd eeee ffff\n"
        hunks = paragraph_hunks(old, new)
        self.assertEqual(len(hunks), 2, "the untouched middle did not split the hunk")
        self.assertNotIn("-let x = 1;", [l for h in hunks for l in h.body], "untouched code was reported")

    def test_two_changes_one_blank_line_apart_become_one_hunk(self):
        # Separately, each would claim that blank line as its own context, and `git apply` refuses
        # overlapping hunks as firmly as context-free ones. Merged, the blank is interior context.
        hunks = paragraph_hunks("aaaa bbbb\ncccc\n\ndddd eeee\nffff\n", "aaaa bbbb cccc\n\ndddd eeee ffff\n")
        self.assertEqual(len(hunks), 1, "the two regions did not merge")
        self.assertEqual((hunks[0].old_count, hunks[0].new_count), (5, 3))
        self.assertEqual(hunks[0].body.count(" "), 1)

    def test_applying_the_hunks_reproduces_what_a_run_would_write(self):
        # The property the preview lives or dies on: what it shows is what a real run writes. A
        # preview that disagrees with its own run is worse than having none.
        old = "keep\n\naaaa bbbb\ncccc\n\ndddd eeee\nffff\n\ntail\n"
        new = "keep\n\naaaa bbbb cccc\n\ndddd eeee ffff\n\ntail\n"
        rebuilt = old.split("\n")[:-1]
        # Back to front, so a hunk's line numbers still address `rebuilt` when it is applied.
        for hunk in reversed(paragraph_hunks(old, new)):
            kept = [l[1:] for l in hunk.body if not l.startswith("-")]
            at = hunk.old_start - 1
            rebuilt[at : at + hunk.old_count] = kept
        self.assertEqual("\n".join(rebuilt) + "\n", new)


if __name__ == "__main__":
    unittest.main()
