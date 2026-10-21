# Code Blocks

Fenced code blocks with syntax highlighting, line highlighting, line numbers, filenames, and word highlighting — powered by [Shiki](https://shiki.style/) and [rehype-pretty-code](https://rehype-pretty.pages.dev/).

Code blocks use standard Markdown fenced code block syntax with meta strings for additional features.
Copy controls appear when readers hover over or focus a code block.
The Code Blocks and CodeGroup gallery previews use the same syntax highlighting and copy controls.

## API

All features are controlled via the **meta string** after the language identifier:

| Feature | Syntax | Description |
|---------|--------|-------------|
| Line highlighting | `{2,4-6}` | Highlight specific lines. Commas separate individual lines, dashes define ranges. |
| Line numbers | `showLineNumbers` | Display line numbers in the gutter. |
| Filename | `filename="app.py"` | Display a filename header above the code block. |
| Word highlighting | `"print"` | Highlight all occurrences of a specific word or string. |
| Disable copy | `copy=false` | Hide the hover and focus copy control for a specific block. |

Features can be combined in any order on the same code block.

## Example

### Line highlighting

Highlight specific lines with `{lines}` after the language:

````md
```python {2,4-6}
import os
import sys  # highlighted
from pathlib import Path
def main():  # highlighted
    print("hello")  # highlighted
    return 0  # highlighted
```
````

```python {2,4-6}
import os
import sys  # highlighted
from pathlib import Path
def main():  # highlighted
    print("hello")  # highlighted
    return 0  # highlighted
```

### Line numbers

Add `showLineNumbers` to display line numbers:

````md
```python showLineNumbers
import os
import sys
from pathlib import Path
```
````

```python showLineNumbers
import os
import sys
from pathlib import Path
```

### Filename

Add `filename="..."` to show a file header:

````md
```python filename="main.py"
def main():
    print("Hello, world!")
```
````

```python filename="main.py"
def main():
    print("Hello, world!")
```

### Word highlighting

Highlight specific words with quoted strings:

````md
```python "print"
import os
print("hello")
print("world")
```
````

```python "print"
import os
print("hello")
print("world")
```

### Combined features

All meta features work together:

````md
```python {2} showLineNumbers filename="app.py"
import os
import sys  # highlighted line, with line numbers and filename
from pathlib import Path
```
````

```python {2} showLineNumbers filename="app.py"
import os
import sys  # highlighted line, with line numbers and filename
from pathlib import Path
```
