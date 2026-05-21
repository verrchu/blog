---
title: "Think twice before deriving serde"
date: 2026-05-21
description: "Deriving Serialize/Deserialize locks a type into a single representation — sometimes that's the wrong default."
---

[serde]: https://github.com/serde-rs/serde
[serialize]: https://docs.rs/serde/latest/serde/trait.Serialize.html
[deserialize]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[serde-with]: https://docs.rs/serde_with/
[serde-as-macro]: https://docs.rs/serde_with/latest/serde_with/attr.serde_as.html
[serialize-as]: https://docs.rs/serde_with/latest/serde_with/trait.SerializeAs.html
[deserialize-as]: https://docs.rs/serde_with/latest/serde_with/trait.DeserializeAs.html
[src]: https://github.com/verrchu/blog/tree/main/content/3-think-twice-before-deriving-serde

[serde] is a great library, and part of what makes it so great in my opinion is that we can
simply derive [`Serialize`][serialize]/[`Deserialize`][deserialize] for a type and that's
it - our type is (de)serilizable.

However, sometimes it probably is not a good idea to just slap the derive onto a certain type
since we can have only one impl of a trait for a type, which is exactly what the derive generates
for us. One impl means that we have only one way to (de)serialize the type.

Part of why this matters is that serde is, in a sense, *too* easy to use — the derives just
work, so types sometimes end up `Serialize`/`Deserialize` more or less by default because of
how easy it is to just add `Serialize, Deserialize` to somethinf liek `#[derive(Debug, Clone)]`.
But then some other piece of code (or some other system entirely) starts relying on the format
that somewhat "accidental" derive produces, and there is no easy going back form that.

Say we have some struct that represents a core domain component. We want to store its
representation as `jsonb` (or similar) in our database, and we also want to serve it to the
frontend. Now there are two consumers of the struct's "representation," and if we just derive
`Serialize` onto it we have only one way to represent it — at which point we might spend real
effort trying to come up with a representation that fits both needs, for no reason other than
the mental model of "single derive, single repr" pushing us into that frame.

I know this trap is real because I got stuck in it at least once. The thing that broke me out was a
colleague suggesting that I literally just add a `to_frontend_json(&self) -> serde_json::Value` method
on the type and use the `serde_json::json!`  macro within it to construct a frontend-specific JSON value.
At the time it seemed very y unusual to me since it was going arund serde in a way but this does gives us
two independent representations without the need to make any compromises which we would otherwise
have to make if we had a single shared one.

{{< code src="examples/domain-object.rs" lang="rust" >}}

The `to_*_json` approach above is a solution, but it's rather unconventional. The textbook approach
is to add a dedicated DTO type per consumer — something like a `DomainObjectFe` with an
`impl From<DomainObject>` to build it — and derive `Serialize` on *that*. This is the standard,
well-understood approach to the "domain model vs. API model" split, and most of the time it's
exactly what we should reach for:

{{< code src="examples/domain-object-fe.rs" lang="rust" >}}

Worth noting: both this DTO approach and the `to_*_json` approach above pay an allocation cost
that we wouldn't pay if we'd just derived `Serialize` on the core type directly — `serde`
streams the struct field-by-field to the writer with no intermediate allocation. The DTO path
reallocates the inner `Vec`; the `to_*_json` path is strictly worse because every nested map
and string in `serde_json::Value` is its own allocation. In most applications this overhead is
irrelevant, but in Rust — where we *could* have gotten zero-cost serialization — knowing that
we paid for it just to decouple the wire format from the in-memory layout might *feel* wrong
even when it doesn't actually matter.

## Structs vs. enums

For dealing with structs I would probably say that having a separate struct for each
representation is correct.

But there is one more thing: enums. And I am talking about plain enums — like

```rust
enum Color {
    Red,
    Blue,
    Green,
}
```

Perhaps here also we could have a `ColorFrontend` enum and a `ColorDb` enum, and that would be
correct, and performance-wise it would not be costly at all to convert from one to another since
they are all `Copy`. *But* it is just sooo tempting to just slap a derive onto the enum and have
something like

```rust
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Color {
    Red,
    Blue,
    Green,
}
```

And that `rename_all = "snake_case"` is where the problem is — it bakes a single string
representation into the type.

## `serde_with` for enums

For enums I think it is a viable option to *not* have a consumer-specific enum version, but
instead have separate serializer/deserializer implementations via [`serde_with`][serde-with],
which provides [`SerializeAs`][serialize-as] / [`DeserializeAs`][deserialize-as] traits and the
[`#[serde_as]`][serde-as-macro] macro, which allows injecting specialized
serializers/deserializers onto specific fields of a struct.

So for `Color` we could have a `ColorUppercase` marker type and implement
`SerializeAs`/`DeserializeAs` for it, and then have:

```rust
#[serde_as]
#[derive(Deserialize, Serialize)]
struct Test {
    #[serde_as(as = "ColorUppercase")]
    color: Color,
}
```

Now we are able to control the representation on a case-by-case basis with a single extra field
attribute.

Unfortunately, the implementation for the specialized serializer/deserializer is done by hand,
and implementing serde serializers/deserializers by hand is a nightmare for complex enough
structures. For plain enums it is easy enough — that's why I recommend it for enums. For structs
I'd say that having per-consumer `to_*_json` methods implemented by hand is more practical for
the serialization path (and in my experience the serialization path comes up more often than the
deserialization path).
