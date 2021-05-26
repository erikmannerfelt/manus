# manus — A manuscript helper to simplify writing good papers. 

Manus is an early work in progress.
All contributions and suggestions are welcome!

Please read the documentation at: XXX


## Installation

```bash
cargo install manus
```


## Simple CLI usage

```bash
manus build main.tex  # This will build a main.pdf
```
will use [tectonic](https://github.com/tectonic-typesetting/tectonic) to build the file called
`main.tex`, with some slightly improved error messages.

When submitting manuscripts to academic journals, there is often a requirement to only have one
source `TeX` file. If the manuscript is written with e.g. multiple chapters
(`\input{introduction.tex}` etc.), this can be merged easily with `manus`:
```bash
manus merge main.tex > merged_text.tex
```

## Templating
The most promiment functionality of `manus` is bridging `TeX` and
[handlebars](https://handlebarsjs.com/); a powerful templating system to separate text and
data.

### Pure-LaTeX example
```tex
\documentclass{article}

\begin{document}
We used 85819 separate measurements to find the value of 58.242$\pm$0.011 units.
\end{document}
```
Note that data and text are quite easily separated here.
Imagine, however, a more complex example of ten pages of text and numbers, and a reviewer's
evil comments suddenly called for revision of half the numbers in the manuscript.
How will you know that you managed to change all of them!?

### Templating example
In `manus` the above example would consist of two files; one for data and one for text.

The data file can be called `data.toml`:
```toml
n_measurements = 85819  # This is how many measurements we have right now, but it may change!

resultant_value = 58.242   # The value (which might change?)
resultant_value_pm = 0.011  # The error of the value
```
And the text; `main.tex`:
```tex
\documentclass{article}

\begin{document}
We used {{n_measurements}} separate measurements to find the value of {{pm resultant_value}}
units.
\end{document}
```
An can be built with:
```bash
manus build -d data.toml main.tex  # This will build a main.pdf
```

Now, we have moved all of our data to a separate machine-readable file.
This has many implications:
1. Data are easily revised throughout the text, so updating results along the way is simple.
2. The supported data formats (JSON and TOML) are machine-readable, meaning they can be created
   automatically from any script written in python, rust, julia etc. "Hardcoding" values can
   theoretically be avoided completely!
3. (See below) Helpers can reduce data repetition by doing simple arithmetic and/or formatting
   for you.


### Template helpers

#### pm --- plus-minus
Arguments:
* `decimal`: Optional. The decimal to round both values to.
* `key`: The key to print and find a corresponding `_pm` key for.

As you saw in the example above, the `n_measurements` key could be fetched from the data file
by simply writing `{{n_measurements}}` in the `TeX` file.
The `resultant_value` has an associated error (could have been called `resultant_error`),
whereby we would write `{{resultant_value}}$\pm${{resultant_error}}`.
This is quite repetitive, however, so a helper `pm` exists to simplify this.

If `{{pm anykey}}` is written, the helper will look for an associated error key: `anykey_pm`.
In the case above, this would be `resultant_value_pm`.


If we want to round both the value and its error, the `decimal` optional argument can be used:

```tex
{{pm 2 resultant_value}}
```
renders to:

```tex
58.24$\pm$0.01
```

#### round --- Round a value to the nearest decimal
Arguments:
* `decimal`: Optional. The decimal to round a value to. Defaults to 0 (integer)
* `value`: The value to round

```tex
{{round resultant_value}}
{{round 2 resultant_value}}
{{round -1 resultant_value}}  % This will round upwards (to the nearest 10)
```
renders to:
```tex
58
58.24
60
```

#### roundup --- Round a value to the nearest power of ten
Arguments:
* `power`: Optional. The power of ten to round toward. Defaults to 0 (integer)
* `value`: The value to round

`roundup` is the same as `round`, only with an inverted sign.

```tex
{{roundup resultant_value}}
{{roundup 1 resultant_value}}
{{roundup -1 resultant_value}}  % This will round downwards (to the nearest decimal)
```
renders to:
```tex
58
60
58.2
```

#### sep --- Add thousand-separators around large numbers
Arguments:
* `value`: The value to make more readable.

**Requires** a key in the data called `separator` which will be used to separate the values.

With `separator = '\,'` (a comma-sized whitespace):

```tex
{{sep n_measurements}}
```
renders to:
```tex
85\,819
```
which looks approximately like '85 819' when rendered into the PDF.

#### upper/lower --- Convert to upper-/lowercase
Arguments:
* `string`: A string to modify

With the key-value pair: `abbreviation = "aBc"` in the `data.toml`:
```tex
{{lower abbreviation}}
{{upper abbreviation}}
```
renders to:
```tex
abc
ABC
```

#### pow --- X to the power of Y
Arguments:
* `value`: The value to raise.
* `exponent`: The exponent to raise a value to

```tex
{{pow 10 8}}
```
renders to:
```tex
100000000
```

#### Chaining helpers

Helpers can be chained using parantheses:

```tex
{{sep (pow 10 8)}}
```
renders to:

```tex
100\,000\,000
```

## Expressions
The "in-`TeX`" helpers are great for small one-time formatting, but expressions in `manus` take
the next step.

Writing `"expr: "` as a value in the data file will evaluate that expression before rendering.

With a `data.toml`:
```toml
n_total_snacks = 2042
n_eaten_snacks = 1567

n_remaining_snacks = "expr: n_total_snacks - n_eaten_snacks"
n_remaining_percentage = "expr: round(100 * n_remaining_snacks / n_total_snacks)"
```
Since `n_eaten_snacks` and `n_remaining_snacks` are always related to each other, and they will change if we eat one more, it's great to define one as a function of the other, instead of "hardcoding" both.

Note that `n_remaining_percentage` depends on an expression (`n_remaining_snacks`), which
is solved by recursively evaluating the independent expressions first, before the
dependent expressions.
If two expressions are dependent on each other (a circular dependency), this will raise a
descriptive recursion error.

### Expression functions
**NOTE**: As of right now (26 May 2021), the underyling expression evaluation engine cannot
understand negative signs properly. `-1` needs to be written as `0-1`, unfortunately! Hopefully
this will change soon.

#### round
Arguments:
* `value`: The value to round.
* `decimal`: Optional. The number of decimals to round to. Defaults to 0.

with the `data.toml`:
```toml
exact_value = 1883.8090928305920395
rounded_value = "expr: round(exact_value, 2)"
integer_value = "expr: round(exact_value)"
```
becomes this in the rendering stage:
```toml
exact_value = 1883.8090928305920395
rounded_value = 1883.81
integer_value = 1884
```

#### pow --- Raise X to Y
* `value`: The value to raise to an exponent.
* `exponent`: The exponent to raise the value to.

```text
"expr: pow(5, 2)"
```
renders to:
```text
25
```

#### E --- Power of ten

`E(x)` is the same as `pow(10, x)`, but a bit shorter:

```text
"expr: 3 * E(2)"
```
renders to:
```text
300
```


## Conversions

Converting a `manus`-flavoured `TeX` into pure `TeX` is done simply:

```bash
manus convert main.tex > boring_version_of_main.tex
```
where the `--format` argument implicitly defaults to `tex`.



## Advanced: Piping

For advanced users, the concept of UNIX piping is embraced with `manus`.

The `-` symbol is used for specifying what to read from stdin:
```bash
curl https://example.com/my_json_data | manus build --data - main.tex
```
Currently, only JSON is supported from pipes.

If, for some reason, we want to use another compiler than `tectonic`, we can pipe the converted
`tex` text data to it:
```bash
manus convert --data=data.toml main.tex | another_tex_compiler
```
