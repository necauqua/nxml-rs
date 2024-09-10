use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Brace,
    Expr, Ident, LitStr, Result, Token,
};

enum RefDeref {
    Ref(Token![&]),
    Deref(Token![*]),
    Mut(Token![mut]),
}

struct RefsDerefs {
    refs: Vec<RefDeref>,
}

impl Parse for RefsDerefs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut refs = Vec::new();
        while !input.is_empty() {
            if let Some(r) = input.parse()? {
                refs.push(RefDeref::Ref(r));
            } else if let Some(r) = input.parse()? {
                refs.push(RefDeref::Deref(r));
            } else if let Some(r) = input.parse()? {
                refs.push(RefDeref::Mut(r));
            } else {
                break;
            }
        }
        Ok(RefsDerefs { refs })
    }
}

impl ToTokens for RefsDerefs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for ref_deref in &self.refs {
            match ref_deref {
                RefDeref::Ref(r) => r.to_tokens(tokens),
                RefDeref::Deref(d) => d.to_tokens(tokens),
                RefDeref::Mut(m) => m.to_tokens(tokens),
            }
        }
    }
}

enum NxmlAttr {
    Literal(Ident, LitStr),
    Expr(Ident, Expr),
    Shortcut(RefsDerefs, Ident),
}

impl Parse for NxmlAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let Some(ident) = input.parse()? else {
            // capture the return Err from the macro
            let content = (|| {
                let content;
                braced!(content in input);
                Ok(content)
            })()
            .map_err(|_| input.error("expected attribute name or identifier in curly braces"))?;

            return Ok(NxmlAttr::Shortcut(content.parse()?, content.parse()?));
        };

        input.parse::<Token![=]>()?;

        if let Some(lit) = input.parse()? {
            return Ok(NxmlAttr::Literal(ident, lit));
        }

        // and again..
        let content = (|| {
            let content;
            braced!(content in input);
            Ok(content)
        })()
        .map_err(|_| input.error("expected a string literal or an expression in curly braces"))?;
        Ok(NxmlAttr::Expr(ident, content.parse()?))
    }
}

enum TextPart {
    Static(String),
    Expr(Expr),
}

enum NxmlFinish {
    SelfClosing,
    Closing {
        text_content: Vec<TextPart>,
        children: Vec<NxmlInput>,
        name: Ident,
    },
}

impl Parse for NxmlFinish {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![/]) && input.peek2(Token![>]) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            return Ok(NxmlFinish::SelfClosing);
        }

        input.parse::<Token![>]>()?;

        let mut children = Vec::new();
        let mut text_content = Vec::new();

        while !(input.peek(Token![<]) && input.peek2(Token![/])) {
            if let Some(lit) = input.parse::<Option<LitStr>>()? {
                text_content.push(TextPart::Static(lit.value()));
                continue;
            }
            if let Some(ident) = input.parse::<Option<Ident>>()? {
                text_content.push(TextPart::Static(ident.to_string()));
                continue;
            }
            if input.peek(Brace) {
                let content;
                braced!(content in input);
                text_content.push(TextPart::Expr(content.parse()?));
                continue;
            }
            if input.peek(Token![<]) {
                children.push(input.parse()?);
                continue;
            }
            return Err(input.error(
                "expected a string literal, an expression in curly braces or a child element",
            ));
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;

        let name = input.parse()?;

        input.parse::<Token![>]>()?;

        Ok(Self::Closing {
            text_content,
            children,
            name,
        })
    }
}

struct NxmlInput {
    name: Ident,
    attrs: Vec<NxmlAttr>,
    finish: NxmlFinish,
}

impl Parse for NxmlInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![<]>()?;

        let name = input.parse()?;

        Ok(NxmlInput {
            attrs: {
                let mut attrs = Vec::new();
                while !(input.peek(Token![>]) || input.peek(Token![/]) && input.peek2(Token![>])) {
                    attrs.push(input.parse()?);
                }
                attrs
            },
            finish: {
                let finish = input.parse()?;
                if let NxmlFinish::Closing { name: end_name, .. } = &finish {
                    // the intellijRulezz thing does not seem to have an appreciable effect
                    // (trying to bait rust-analyzer to autocomplete the closing tag)
                    if *end_name != name && end_name != "intellijRulezz" {
                        let /* mut */ err = syn::Error::new_spanned(
                            end_name,
                            "expected closing tag to match opening tag",
                        );
                        // this creates a huge mess for some reason
                        // seems like rust isn't happy with multiple compile_error!s emitted from a
                        // macro
                        // err.combine(syn::Error::new_spanned(name, "opening tag here"));
                        return Err(err);
                    }
                }
                finish
            },
            name,
        })
    }
}

fn codegen(input: &NxmlInput, element: TokenStream2) -> TokenStream2 {
    let name = &input.name;

    let (text_parts, children, end_name) = match &input.finish {
        NxmlFinish::Closing {
            text_content,
            children,
            name,
        } => (&text_content[..], &children[..], name),
        _ => (&[][..], &[][..], name),
    };

    let mut static_text = String::new();
    let mut text_exprs = Vec::new();
    for part in text_parts {
        if !static_text.is_empty() {
            static_text.push(' ');
        }
        match part {
            TextPart::Static(str) => static_text.push_str(str),
            TextPart::Expr(expr) => {
                static_text.push_str("{}");
                text_exprs.push(expr);
            }
        }
    }

    let attrs = input.attrs.iter().map(|attr| match attr {
        NxmlAttr::Literal(ident, value) => quote!(.with_attr(stringify!(#ident), #value)),
        NxmlAttr::Expr(ident, expr) => quote!(.with_attr(stringify!(#ident), #expr)),
        NxmlAttr::Shortcut(r, ident) => quote!(.with_attr(stringify!(#ident), #r #ident)),
    });

    let text_content = if text_exprs.is_empty() {
        if !static_text.is_empty() {
            quote!(.with_text(#static_text))
        } else {
            quote!()
        }
    } else {
        quote!(.with_text(format!(#static_text, #(#text_exprs),*)))
    };

    let children = children.iter().map(|child| {
        let tokens = codegen(child, element.clone());
        quote!(.with_child(#tokens))
    });

    quote!({
        #[allow(non_camel_case_types)]
        struct #name;
        // can use rust-analyzer rename action to change the tag in sync, and
        // goto reference to jump between them
        // (name and end_name are always same, but we (and RA) care about spans)
        let _: #name = #end_name;

        ::nxml_rs::#element::new(stringify!(#name))
            #text_content
            #(#attrs)*
            #(#children)*
    })
}

/// Creates an [`Element`](struct.Element.html) from an XML-like syntax.
///
/// # Example
/// ```rust
/// # use nxml_rs::*;
/// # let outside_var = 42;
/// # let shortcut_name = "minä";
/// # let element =
/// nxml! {
///     <Entity>
///         <SomeComponent name="comp" value={outside_var} {shortcut_name} />
///         <BareTextIsMeh>
///             bare words "(idents only)" or
///             "string literals or"
///             {"exprs"}
///             "are format!'ed into a single string"
///             "(when an expr occurs the zerocopy breaks and we have a Cow::Owned)"
///         </BareTextIsMeh>
///     </Entity>
/// };
///
/// # assert_eq!(element.to_string(), "<Entity><SomeComponent name=\"comp\" value=\"42\" shortcut_name=\"minä\"/><BareTextIsMeh>bare words (idents only) or string literals or exprs are format!'ed into a single string (when an expr occurs the zerocopy breaks and we have a Cow::Owned)</BareTextIsMeh></Entity>");
/// ```
#[proc_macro]
pub fn nxml(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NxmlInput);
    codegen(&input, quote!(Element)).into()
}

/// Creates an [`ElementRef`](struct.ElementRef.html) from an
/// XML-like syntax.
///
/// # Examples
///
/// With no expressions, the result is `ElementRef<'static>`:
/// ```rust
/// # use nxml_rs::*;
/// # fn assert_static(_: ElementRef<'static>) {}
/// assert_static(nxml_ref!(<Entity prop="static" />));
/// ```
///
/// The lifetime is narrowed down to the shortest one of given expressions:
/// ```compile_fail
/// # use nxml_rs::*;
/// # fn assert_static(_: ElementRef<'static>) {}
/// let prop = String::from("value");
///
/// let element = nxml_ref!(<Entity {&prop} />); // borrowed value does not live long enough..
///
/// assert_static(element); // ..argument requires that `prop` is borrowed for `'static`
/// ```
///
/// And, unlike [`nxml!`](macro.nxml.html), the expressions must be `&str`:
/// ```compile_fail
/// # use nxml_rs::*;
/// let prop = 42;
/// nxml_ref!(<Entity {prop} />); // expected `&str`, found integer
#[proc_macro]
pub fn nxml_ref(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NxmlInput);
    codegen(&input, quote!(ElementRef)).into()
}

struct NxmlMultiInput {
    children: Vec<NxmlInput>,
}

impl Parse for NxmlMultiInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut children = Vec::new();
        while !(input.peek(Token![<]) && input.peek2(Token![/]) || input.is_empty()) {
            children.push(input.parse()?);
        }
        Ok(NxmlMultiInput { children })
    }
}

/// Creates a list of [`Element`](struct.Element.html) from an
/// XML-like syntax.
///
/// This is equivalent to calling [`nxml!`](macro.nxml.html) multiple times
/// inside of a `vec!` macro (or doing `nxml!(<root>...</root>).children`).
/// # Example
/// ```rust
/// # use nxml_rs::*;
/// let elements = nxmls!(<a/><b/><c/>);
///
/// assert_eq!(elements.len(), 3);
/// ```
#[proc_macro]
pub fn nxmls(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NxmlMultiInput);
    let items = input
        .children
        .iter()
        .map(|child| codegen(child, quote!(Element)));
    quote!(vec![#(#items),*]).into()
}

/// Creates a list of [`ElementRef`](struct.Element.html) from an
/// XML-like syntax.
///
/// This is equivalent to calling [`nxml_ref!`](macro.nxml_ref.html) multiple
/// times inside of a `vec!` macro (or doing
/// `nxml_refs!(<root>...</root>).children`).
/// # Example
/// ```rust
/// # use nxml_rs::*;
/// let elements = nxml_refs!(<a/><b/><c/>);
///
/// assert_eq!(elements.len(), 3);
/// ```
#[proc_macro]
pub fn nxml_refs(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as NxmlMultiInput);
    let items = input
        .children
        .iter()
        .map(|child| codegen(child, quote!(ElementRef)));
    quote!(vec![#(#items),*]).into()
}
