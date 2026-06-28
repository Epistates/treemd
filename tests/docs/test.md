# Link Testing Document

## Table of Contents
- [Markdown Links](#markdown-links)
- [WikiLinks](#wikilinks)
- [Anchor Tests](#anchor-tests)
- [Edge Cases](#edge-cases)

---

## Markdown Links

### Basic Markdown Links (No Extension)
- [Link to simple file](simple-file)
- [Link to file with spaces](file with spaces)
- [Link to hyphenated-file](hyphenated-file)

### Markdown Links (With .md Extension)
- [Link to simple file](simple-file.md)
- [Link to file with spaces](file with spaces.md)
- [Link to hyphenated-file](hyphenated-file.md)

### Markdown Links with Anchors (No Extension)
- [Link to heading in other file](other-file#section-one)
- [Link to heading with spaces](other-file#heading with spaces)
- [Link to heading in file with spaces](file with spaces#introduction)

### Markdown Links with Anchors (With .md Extension)
- [Link to heading in other file](other-file.md#section-one)
- [Link to heading with spaces](other-file.md#heading-with-spaces)
- [Link to heading in file with spaces](file with spaces.md#introduction)

### Markdown Links to Current File Anchors
- [Jump to Markdown Links section](#markdown-links)
- [Jump to WikiLinks section](#wikilinks)
- [Jump to Edge Cases](#edge-cases)

---

## WikiLinks

### Basic WikiLinks (No Extension)
- [[simple-file]]
- [[file with spaces]]
- [[hyphenated-file]]

### WikiLinks (With .md Extension)
- [[simple-file.md]]
- [[file with spaces.md]]
- [[hyphenated-file.md]]

### WikiLinks with Display Text
- [[simple-file|Custom Display Text]]
- [[file with spaces|Another Name]]
- [[hyphenated-file|Different Label]]

### WikiLinks with Anchors (No Extension)
- [[other-file#section-one]]
- [[other-file#heading with spaces]]
- [[file with spaces#introduction]]

### WikiLinks with Anchors (With .md Extension)
- [[other-file.md#section-one]]
- [[file with spaces.md#introduction]]
- [[hyphenated-file.md#conclusion]]

### WikiLinks with Anchors and Display Text
- [[other-file#section-one|Jump to Section One]]
- [[file with spaces#introduction|Read the Intro]]
- [[hyphenated-file#conclusion|See Conclusion]]

### WikiLinks to Current File Anchors
- [[#markdown-links]]
- [[#wikilinks]]
- [[#anchor-tests]]

---

## Anchor Tests

### Various Anchor Formats in Current Document
- [Standard kebab-case anchor](#anchor-tests)
- [Anchor with numbers](#heading-123)
- [Multiple word anchor](#this-is-a-long-heading)

### Cross-Document Anchors
- [Other doc, standard anchor](other-doc#standard-section)
- [Other doc, complex anchor](other-doc#complex-heading-with-123-numbers)
- [[other-doc#standard-section|WikiLink to anchor]]

---

## Edge Cases

### Special Characters and Encoding
- [File with underscores](file_with_underscores)
- [File with numbers](file123)
- [[file_with_underscores]]
- [[file123]]

### Mixed Format Links
- [Link to file](simple-file) and [[simple-file|WikiLink to same file]]
- Both formats: [Markdown](other-file#intro) vs [[other-file#intro|WikiLink]]

### Relative Paths
- [Subdirectory file](./subfolder/nested-file)
- [Parent directory file](../parent-file)
- [Deep nested path](./folder/subfolder/deep-file.md)

### URL-Encoded Spaces
- [File with %20](file%20with%20spaces)
- [Anchor with %20](other-file#heading%20with%20spaces)

### Case Sensitivity Tests
- [lowercase-file](lowercase-file)
- [UPPERCASE-FILE](UPPERCASE-FILE)
- [MixedCase-File](MixedCase-File)
- [[lowercase-file]]
- [[UPPERCASE-FILE]]
- [[MixedCase-File]]

---

## Heading 123

This heading tests anchors with numbers.

## This Is A Long Heading

This tests multi-word headings.

## Complex-Heading_With 123 Numbers!

This tests special characters in headings.
