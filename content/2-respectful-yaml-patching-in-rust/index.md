---
title: "\"Respectful\" YAML patching in Rust"
date: 2026-05-08
description: "Finding a Rust library that can patch YAML files without losing their comments or formatting."
---

[archive]: https://github.com/dtolnay/serde-yaml/commit/3ba8462f7d3b603d832e0daeb6cfc7168a673d7a
[saphyr-handoff]: https://github.com/Ethiraric/yaml-rust2/issues/21#issuecomment-2204512056
[yamlpath-claim]: https://github.com/zizmorcore/zizmor/blob/main/crates/yamlpath/src/lib.rs#L1
[ry-demo]: https://github.com/elioetibr/rust-yaml/blob/main/examples/comment_preservation_demo.rs
[crates-io]: https://crates.io/
[lib-rs]: https://lib.rs/
[src]: https://github.com/verrchu/blog/tree/main/content/2-respectful-yaml-patching-in-rust

Patching a YAML file programmatically is straightforward in principle: parse, modify,
serialize. Ideally the process should also be *respectful* — that is, preserve the
following properties of the initial file:

1. **Formatting.** The same YAML value can be represented in multiple ways: how
   mappings and lists are indented, whether blank lines separate sections, how strings
   are quoted, and so on. For example, a list can be represented in *block* style

   ```yaml
   items:
     - 1
     - 2
     - 3
   ```

   or in *flow* style

   ```yaml
   items: [1, 2, 3]
   ```

   A general-purpose YAML library typically picks one canonical form when serializing
   and applies it to the entire document.

2. **Comments.** One of YAML's ergonomic advantages is that a value can have an
   associated inline note explaining *why* it's set the way it is. Comments are
   typically erased at the deserialization stage and therefore have no chance to be
   serialized back.

Losing either property hurts. A dropped comment effectively loses historical context.
Mangled formatting can render the resulting file invalid, or wipe out a layout that
was carefully chosen for a specific situation (e.g. turn an intentional flow list
into a block list).

Reaching for a popular general-purpose YAML library is the obvious move, but none of
them preserve both:

- [**`serde_yaml`**](https://github.com/dtolnay/serde-yaml) is
  [no longer maintained][archive], and the
  [feature request](https://github.com/dtolnay/serde-yaml/issues/145) was declined as
  out of scope long before that.
- [**`yaml-rust2`**](https://github.com/Ethiraric/yaml-rust2) doesn't preserve comments;
  the [feature request](https://github.com/Ethiraric/yaml-rust2/issues/21) was closed
  with the note that the work [would happen in saphyr][saphyr-handoff] instead.
- [**`saphyr`**](https://github.com/saphyr-rs/saphyr) is yaml-rust2's spiritual successor
  by the same maintainer; comment support is
  [planned](https://github.com/saphyr-rs/saphyr/issues/12) but not yet shipped.

So a more niche tool is needed.

## The candidates

A search of [crates.io][crates-io] and [lib.rs][lib-rs] for libraries that claim
comment preservation turns up four candidates:

- [**`yamlpath`**](https://github.com/zizmorcore/zizmor/tree/main/crates/yamlpath) +
  [**`yamlpatch`**](https://github.com/zizmorcore/zizmor/tree/main/crates/yamlpatch) —
  [comment- and format-preserving][yamlpath-claim] routing (`yamlpath`) and patch
  operations (`yamlpatch`).
- [**`yaml-edit`**](https://github.com/jelmer/yaml-edit) — per its
  [README](https://github.com/jelmer/yaml-edit#readme), preserves *formatting,
  comments, and whitespace*.
- [**`rust-yaml`**](https://github.com/elioetibr/rust-yaml) — README has a dedicated
  [Comment Preservation](https://github.com/elioetibr/rust-yaml#-comment-preservation-implemented)
  section.
- [**`yamp`**](https://github.com/sanjeevprasad/yamp) — README lists
  [comment preservation](https://github.com/sanjeevprasad/yamp#1-comment-preservation-first)
  as one of the project's design goals.

## The experiment

The example below uses a simplified config for a crypto trading bot. The assets
are grouped into named groups with a catch-all `default` group:

```yaml
# outer comment
asset_groups:
  majors:    # majors group comment
    - BTC
    - ETH
    - SOL
  # memes group outer comment
  memes:
    -  DOGE       # asset comment
    - PEPE
  default:
    # default group inner comment
    - 1INCH
    - ATOM
    - LINK
```

The toy CLI used here supports two operations:

- `list-assets X,Y,Z` — append the listed assets to the `default` group, in alphabetical
  order.
- `delist-assets X,Y,Z` — remove the listed assets from whichever group they live in. If
  a group goes empty, drop the group entirely.

## Listing assets

The first test is a single `list-assets` invocation with four assets, picked to
exercise three cases at once:

```text
list-assets 1INCH,BTC,XRP,BNB
```

- `1INCH` is already in `default` → no-op.
- `BTC` is already in `majors` → also no-op.
- `XRP` and `BNB` are new and should land in `default`, alphabetically sorted alongside
  the existing items.

The expected output:

```yaml
# outer comment
asset_groups:
  majors:    # majors group comment
    - BTC
    - ETH
    - SOL
  # memes group outer comment
  memes:
    -  DOGE       # asset comment
    - PEPE
  default:
    # default group inner comment
    - 1INCH
    - ATOM
    - BNB
    - LINK
    - XRP
```

<details>
<summary><strong><code>yamlpath</code> + <code>yamlpatch</code></strong> — exact match</summary>

```yaml
# outer comment
asset_groups:
  majors:    # majors group comment
    - BTC
    - ETH
    - SOL
  # memes group outer comment
  memes:
    -  DOGE       # asset comment
    - PEPE
  default:
    # default group inner comment
    - 1INCH
    - ATOM
    - BNB
    - LINK
    - XRP
```

</details>

<details>
<summary><strong><code>yaml-edit</code></strong> —
outer comment dropped, "default" misindented</summary>

```yaml
asset_groups:
  majors:    # majors group comment
    - BTC
    - ETH
    - SOL
  # memes group outer comment
  memes:
    -  DOGE       # asset comment
    - PEPE
  default:
    # default group inner comment
                - 1INCH
    - ATOM
    - BNB
    - LINK
    - XRP
```

</details>

<details>
<summary><strong><code>rust-yaml</code></strong> — multiple issues, disqualified</summary>

```yaml
# outer comment
asset_groups: 
  majors: 
    - BTC
    - ETH
    - SOL
  memes: 
    - DOGE
    - PEPE
  default: 
    - 1
    - 1INCH
    - ATOM
    - BNB
    - INCH
    - LINK
    - XRP
# majors group comment

# outer comment

# outer comment

# majors group comment
```

The comments are scattered (some end up at the bottom of the file, some duplicated),
`1INCH` is split into two list items (`- 1` and `- INCH`), and the deliberate
whitespace and inline comment on `DOGE` are both lost.

The library's own [`comment_preservation_demo.rs`][ry-demo] exhibits the
same comment-scattering behavior when run unmodified.

</details>

<details>
<summary><strong><code>yamp</code></strong> — parsing issues, disqualified</summary>

No output is shown here because some of the comments in the input confuse yamp's
parser.

</details>

`list-assets` is the easier of the two operations since it only touches a single group
and only adds. `yamlpath` + `yamlpatch` round-trip the file exactly.
`yaml-edit` does violate both properties, but not severely enough to disqualify it on
this test alone.

## Delisting assets

`delist-assets` is the more demanding operation: any group can be modified, any asset
can be removed, groups can be removed entirely. The test:

```text
delist-assets DOGE,PEPE,BTC,SOL,ATOM,SHIB
```

That covers every interesting case at once:

- `DOGE` and `PEPE` are both members of `memes`. Removing both should empty the group,
  which means the whole `memes` group has to be removed.
- `BTC` and `SOL` come out of `majors`, leaving it with only `ETH`.
- `ATOM` is removed from `default`.
- `SHIB` isn't in the file at all; should be a no-op.

The expected output:

```yaml
# outer comment
asset_groups:
  majors:    # majors group comment
    - ETH
  default:
    # default group inner comment
    - 1INCH
    - LINK
```

<details>
<summary><strong><code>yamlpath</code> + <code>yamlpatch</code></strong> —
almost, a single comment rearranged</summary>

```yaml
# outer comment
asset_groups:
  majors:    # majors group comment
    - ETH  # memes group outer comment
  default:
    # default group inner comment
    - 1INCH
    - LINK
```

When the now-empty `memes:` key is removed, the standalone comment that was sitting
on the line above it doesn't get removed with it. Instead it migrates onto the
nearest surviving content line as an inline comment. The output is valid YAML and
no comment is lost, but the comment is now attached to the wrong list item.

</details>

<details>
<summary><strong><code>yaml-edit</code></strong> — logical structure changed, disqualified</summary>

```yaml
asset_groups:
  majors:    # majors group comment
                - ETH
  # memes group outer comment
    default:
    # default group inner comment
                - 1INCH
    - LINK
```

The indentation shift on `default:` is not just cosmetic: `default` is now nested
*inside* `majors` rather than being a sibling. The two top-level groups have
collapsed into one.

</details>

`yamlpath` + `yamlpatch` produces valid YAML, but the stranded comment is a
violation of the "respectfulness" properties — which is probably not a dealbreaker
given the state of other libraries.

## The winner

`yamlpath` + `yamlpatch` is currently the best of the available options. It is not
perfect, but it is actively maintained and it can be made to work with some
workarounds and compromises. Here are some caveats I encountered while trying to make
it work for my actual use case.

### `Op::Replace` doesn't work on sequences

<details>
<summary><code>yamlpatch-replace-list.rs</code></summary>

{{< code src="examples/yamlpatch-replace-list.rs" lang="rust" >}}

</details>

```text
$ cargo run --example yamlpatch-replace-list -- --assets 1INCH,BTC,XRP,BNB
Error: apply patches

Caused by:
    0: YAML query error: input is not valid YAML
    1: input is not valid YAML
```

### Updating a list requires a workaround

Since `Op::Replace` is unusable on sequences, updating a list end-to-end requires a
workaround: append the entire desired list to the end first, then remove the original
items from the front one at a time:

<details>
<summary><code>yamlpatch-rotate-replace-list.rs</code></summary>

{{< code src="examples/yamlpatch-rotate-replace-list.rs" lang="rust" >}}

</details>

```text
$ cargo run --example yamlpatch-rotate-replace-list -- --assets 1INCH,BTC,XRP,BNB
asset_groups:
  default:
    - 1INCH
    - ATOM
    - BNB
    - BTC
    - LINK
    - XRP
```

### It doesn't play well with flow-style lists

<details>
<summary><code>yamlpatch-flow-list.rs</code></summary>

{{< code src="examples/yamlpatch-flow-list.rs" lang="rust" >}}

</details>

```text
$ cargo run --example yamlpatch-flow-list -- --assets 1INCH,BTC,XRP,BNB
Error: apply patches

Caused by:
    Invalid operation: append operation is not permitted against flow sequence route: Route { route: [Key("asset_groups"), Key("default")] }
```

## Conclusion

`yamlpath` + `yamlpatch` is the only option that comes truly close to "respectful"
patching as defined here. It is very much usable in practice, even though it doesn't
cover every case out of the box.

---

*Full accompanying source code can be found [here][src]. Built with `rustc 1.95.0`.
Library versions used: `yamlpath` 1.24.1, `yamlpatch` 1.24.1, `yaml-edit` 0.2.1,
`rust-yaml` 0.0.5, `yamp` 0.1.0.*

