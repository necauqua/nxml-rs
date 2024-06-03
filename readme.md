nxml-rs
===
[![Crates.io](https://img.shields.io/crates/v/nxml-rs)](https://crates.io/crates/nxml-rs)
[![CI](https://github.com/necauqua/nxml-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/necauqua/nxml-rs/actions/workflows/ci.yml)
![License](https://img.shields.io/github/license/necauqua/nxml-rs)
[![discord link](https://img.shields.io/discord/587713408841940993)](https://google.com)

nxml-rs is a [Rust rewrite](https://transitiontech.ca/random/RIIR) of
[NXML](https://github.com/XWitchProject/NXML).

NXML is a pseudo-XML parser that can read and write the oddly formatted and
often invalid XML files from [Noita](https://noitagame.com). NXML (the C#/.NET
version linked before) is itself a port of the XML parser from
[Poro](https://github.com/gummikana/poro), which is used by the custom Falling
Everything engine on which Noita runs.

In short, nxml-rs is an XML parser that is 100% equivalent to Noita's parser. It
is just as non-conformant to the XML specification as Noita itself. It can also
produce semantically equivalent output in the form of a string.

Additionally (just for fun), nxml-rs provides an `nxml!` macro that allows you
to create nxml elements by writing almost-XML (text content requires some extra
characters and is generally poorly supported and not really used by Noita
anyway) directly in your Rust code.

Because it's Rust and thus it was relatively easy to do, the parsing is almost
zero-copy - a single exception being non-continuous bare text,
`<a>hello<b/>world</a>`, which is non-existingly rare and is a preserved quirk
of how noita parser handles that anyway, handled with `Cow`.

### Example

```rust
use nxml_rs::*;

let mut entity = nxml_rs::parse(r#"
    <Entity>
        <LuaComponent
            script_source_file="mods/blah/etc/test.lua"
            execute_every_n_frame="-1">
        </LuaComponent>
        <TestComponent blah="blah" />
    </Entity>
"#).unwrap();

// A little DSL sugar thing, / is alias for .child("name").unwrap(),
// and % is for .attr("name").unwrap()
assert_eq!("-1", &entity / "LuaComponent" % "execute_every_n_frame");

let a_lot = 0;
let speed = 0;

// Make a new element with builder methods
let extra = Element::new("ElectricityComponent")
    .with_attr("energy", a_lot)
    .with_attr("probability_to_heat", "0")
    .with_attr("speed", speed);

// But it's more convenient to use the macro, the builder is rarely ever
// needed (and the following macro expands to above code)
let extra = nxml! {
    <ElectricityComponent energy={a_lot} probability_to_heat="0" {speed} />
};

// Clone everything into owned strings, making it a bit nicer to work with
let mut owned = entity.to_owned();

// A modification - entity.children is just a vec ¯\_(ツ)_/¯
owned.children.insert(1, extra);

// Still has the sugar
assert_eq!("0", &owned / "ElectricityComponent" % "probability_to_heat");

// And can be rendered back into a string (.display() making it pretty-printed)
assert_eq!(owned.display().to_string(), r#"
<Entity>
    <LuaComponent script_source_file="mods/blah/etc/test.lua" execute_every_n_frame="-1"/>
    <ElectricityComponent energy="0" probability_to_heat="0" speed="0"/>
    <TestComponent blah="blah"/>
</Entity>
"#.trim());

// DSL is defined for both of them, and / works with &mut
(&mut owned / "LuaComponent").remove_attr("execute_every_n_frame");
(&mut entity / "LuaComponent").remove_attr("execute_every_n_frame");

entity.children.remove(1);

// The EntityRef can be rendered too
assert_eq!(entity.to_string(), "<Entity><LuaComponent script_source_file=\"mods/blah/etc/test.lua\"/></Entity>");
```

### Cargo features
- `indexmap` - Use `IndexMap` from the `indexmap` crate instead of `HashMap`
  for attributes. This is useful if you want to preserve the order of
  attributes in the output. Enabled by default, you can disable it to shake off
  that one dependency if you don't care about attrubute order.
