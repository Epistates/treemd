# Issue #43 Test Document

This file exercises every fix from v0.5.8. Use it to manually verify all items.

---

## 1. EOF Scroll Test

This section has enough content to require scrolling. Verify that:
- **Down arrow** stops when the last line is at the viewport bottom (not past it)
- **Page Down** stops at the same place
- **End** (`G` or `Fn+Right`) jumps to the bottom and stops
- The scrollbar reaches 100% exactly when the last line is visible

Line 1 of filler content for scroll testing.
Line 2 of filler content for scroll testing.
Line 3 of filler content for scroll testing.
Line 4 of filler content for scroll testing.
Line 5 of filler content for scroll testing.
Line 6 of filler content for scroll testing.
Line 7 of filler content for scroll testing.
Line 8 of filler content for scroll testing.
Line 9 of filler content for scroll testing.
Line 10 of filler content for scroll testing.
Line 11 of filler content for scroll testing.
Line 12 of filler content for scroll testing.
Line 13 of filler content for scroll testing.
Line 14 of filler content for scroll testing.
Line 15 of filler content for scroll testing.
Line 16 of filler content for scroll testing.
Line 17 of filler content for scroll testing.
Line 18 of filler content for scroll testing.
Line 19 of filler content for scroll testing.
Line 20 of filler content for scroll testing.
Line 21 of filler content for scroll testing.
Line 22 of filler content for scroll testing.
Line 23 of filler content for scroll testing.
Line 24 of filler content for scroll testing.
Line 25 of filler content for scroll testing.
Line 26 of filler content for scroll testing.
Line 27 of filler content for scroll testing.
Line 28 of filler content for scroll testing.
Line 29 of filler content for scroll testing.
Line 30 of filler content for scroll testing.
Line 31 of filler content for scroll testing.
Line 32 of filler content for scroll testing.
Line 33 of filler content for scroll testing.
Line 34 of filler content for scroll testing.
Line 35 of filler content for scroll testing.
Line 36 of filler content for scroll testing.
Line 37 of filler content for scroll testing.
Line 38 of filler content for scroll testing.
Line 39 of filler content for scroll testing.
Line 40 of filler content for scroll testing.
Line 41 of filler content for scroll testing.
Line 42 of filler content for scroll testing.
Line 43 of filler content for scroll testing.
Line 44 of filler content for scroll testing.
Line 45 of filler content for scroll testing.
Line 46 of filler content for scroll testing.
Line 47 of filler content for scroll testing.
Line 48 of filler content for scroll testing.
Line 49 of filler content for scroll testing.
Line 50 of filler content for scroll testing.

---

## 2. Protocol Table (Crash Test at Width >= 146)

Resize your terminal to various widths. At 146+ characters wide, this should NOT crash.

| Protocol | Port(s) | Transport | Purpose | Encryption | Key Feature | Common Usage |
|----------|---------|-----------|---------|------------|-------------|--------------|
| HTTP | 80 | TCP | Web browsing | No | Stateless request-response | Websites, APIs |
| HTTPS | 443 | TCP | Secure web | TLS/SSL | Encrypted HTTP | Banking, login pages |
| FTP | 20/21 | TCP | File transfer | Optional (FTPS) | Active/Passive modes | File sharing, uploads |
| SSH | 22 | TCP | Secure shell | Yes (built-in) | Encrypted terminal | Remote admin, tunnels |
| SMTP | 25/587 | TCP | Email sending | STARTTLS | Store-and-forward | Mail servers |
| DNS | 53 | TCP/UDP | Name resolution | DoH/DoT | Hierarchical lookup | Every internet request |
| DHCP | 67/68 | UDP | IP assignment | No | Lease-based allocation | Network autoconfiguration |

### 6-Column URL Protocol Table (from issue)

| Protocol | RFC | `//` | Separator | Parameters (Standardized) | Example URL |
|---|---|---|---|---|---|
| **`http(s):`** | 9110 | Required | `?` `&` | `query`, `search`, `id`, `sort` | `https://example.com?query=term&sort=asc` |
| **`mailto:`** | 6068 | Forbidden | `?` `&` | `to`, `cc`, `bcc`, `subject`, `body` | `mailto:a@b.com?subject=Hello&body=Hi%20You` |
| **`tel:`** | 3966 | Forbidden | `;` | `tn` (name), `phone-context`, `extension` | `tel:+123456;tn=Jane;extension=456` |
| **`sms:`** | 5724 | Forbidden | `?` `&` | `body`, `subject` (MMS), `attachment` | `sms:+123?body=Meeting%20confirmed` |
| **`data:`** | 2397 | Forbidden | `;` `,` | `text/plain`, `charset`, `base64` | `data:text/csv;base64,SGVsbG8=` |
| **`ftp:`** | 1738 | Required | -- | -- | `ftp://user@example.com/file.txt` |
| **`ssh:`** | 4252 | Required | -- | -- | `ssh://user@example.com:22` |
| **`file:`** | 8089 | Optional | -- | -- | `file:///C:/path/file.txt` |
| **`facetime:`** | -- | Forbidden | `?` | `p` (Apple ID token), `t` (timestamp) | `facetime:user@icloud.com?p=token123` |
| **`maps:`** (Apple) | -- | Required | `?` `&` | `q`, `ll`, `z`, `t` (type), `dirflg` | `maps://?q=Paris&t=h&z=12` |

---

## 3. LaTeX Filtering Test

Enable `hide_latex = true` in your config. All LaTeX commands below should be stripped, leaving only readable text.

### Standalone commands (should vanish entirely)

\newpage
\clearpage
\tableofcontents
\maketitle
\centering
\noindent
\bigskip

### Font size commands (should vanish)

\tiny
\scriptsize
\small
\normalsize
\large
\Large
\LARGE
\huge
\Huge

### Setup commands with args (should vanish)

\usepackage[utf8]{inputenc}
\documentclass[12pt]{article}
\setlength{\parindent}{0pt}
\renewcommand{\baselinestretch}{1.5}
\newcommand{\mycommand}{definition}
\pagestyle{fancy}
\thispagestyle{empty}
\pagenumbering{arabic}
\geometry{margin=1in}
\sethlcolor{yellow}
\titlespacing{\section}{0pt}{12pt}{6pt}
\captionsetup{font=small}

### Inline LaTeX (commands stripped, text preserved)

This has \textbf{bold text} and \textit{italic text} and \underline{underlined text} inline.

Here is \textcolor{red}{colored text} and \colorbox{yellow}{highlighted text} in a sentence.

The value of \hl{important stuff} should remain visible.

### Inline commands that should be fully stripped

Text with \fontsize{12}{14} commands and \setlength{\parskip}{1em} more text here.

Some \usepackage[T1]{fontenc} leftover commands \thispagestyle{plain} in prose.

### Bare commands (should vanish, text around them preserved)

Check \Box$ next item and \no skip \yes proceed.

Greek: The angle $\alpha$ approaches $\beta$ as $\gamma$ increases.

Math: $\sum_{i=0}^{n} x_i = \infty$

### Unpaired environments

\begin{center}
This text is centered
\end{center}

\begin{itemize}
Item text here
\end{itemize}

---

## 4. Long Wrapping Content (EOF Scroll + Line Count Accuracy)

This paragraph is intentionally very long to test that the visual line count after word-wrapping is calculated correctly. When word-wrap produces more visual lines than raw lines, the old code would stop scrolling at ~78% because it counted raw lines instead of wrapped lines. This sentence keeps going and going and going and going and going and going and going and going and going and going and going and going and going and going to ensure it wraps multiple times across the viewport width.

Another extremely long paragraph follows to add more wrapped content. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog.

---

## 5. Outline Visibility Test

To test auto-hide outline:
1. Put this file ALONE in a directory (no other .md files)
2. Run `treemd test_issue43.md` -- outline should be hidden
3. Copy another .md file into the same directory
4. Run `treemd test_issue43.md` -- outline should be visible

---

## 6. File Picker Test

To test folder navigation:
1. Press `Ctrl+O` to open file picker
2. Verify subdirectories appear at the bottom with `[DIR]` prefix
3. Press Enter on a directory to navigate into it
4. Press Backspace to go back to parent directory
5. Type `/` to search -- filter applies to both files and directories
6. In search mode with empty query, press Backspace to go to parent

---

## 7. Gapless Cursor Test

1. Press `:` to open the command palette
2. Verify the cursor is a solid white block with no vertical gaps
3. Type some characters and verify cursor remains gapless

---

This is the last line of the test document. Scrolling should stop here.
