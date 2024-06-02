use proc_macro::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Brace,
    Expr, Ident, LitStr, Result, Token,
};

enum NxmlAttr {
    Literal(Ident, LitStr),
    Expr(Ident, Expr),
    Shortcut(Ident),
}

impl Parse for NxmlAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        if !input.peek(Ident) {
            // capture the return Err from the macro
            let content = (|| {
                let content;
                braced!(content in input);
                Ok(content)
            })()
            .map_err(|_| input.error("expected attribute name or identifier in curly braces"))?;

            return Ok(NxmlAttr::Shortcut(content.parse()?));
        }
        let ident = input.parse()?;
        input.parse::<Token![=]>()?;
        if input.peek(LitStr) {
            return Ok(NxmlAttr::Literal(ident, input.parse()?));
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

enum ExprOrStr {
    Expr(Expr),
    Str(LitStr),
}

enum NxmlFinish {
    SelfClosing,
    Closing {
        text_content: Vec<ExprOrStr>,
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
            if input.peek(LitStr) {
                text_content.push(ExprOrStr::Str(input.parse()?));
            } else if input.peek(Brace) {
                let content;
                braced!(content in input);
                text_content.push(ExprOrStr::Expr(content.parse()?));
            } else if input.peek(Token![<]) {
                children.push(input.parse()?);
            } else {
                return Err(input.error(
                    "expected a string literal, an expression in curly braces or a child element",
                ));
            }
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
        let name: Ident = input.parse()?;
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
                    if *end_name != name && end_name != "intellijRulezz" {
                        let /* mut */ err = syn::Error::new_spanned(
                            end_name,
                            "expected closing tag to match opening tag",
                        );
                        // this creates a huge mess for some reason
                        // seems like rust isn't happy with multiple compile_error!s emitted from a
                        // macro err.combine(syn::Error::new_spanned(name,
                        // "opening tag here"));
                        return Err(err);
                    }
                }
                finish
            },
            name,
        })
    }
}

impl ToTokens for NxmlInput {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;

        let attrs = self.attrs.iter().map(|attr| match attr {
            NxmlAttr::Literal(ident, value) => {
                quote!(.with_attr(stringify!(#ident), #value))
            }
            NxmlAttr::Expr(ident, expr) => quote!(.with_attr(stringify!(#ident), #expr)),
            NxmlAttr::Shortcut(ident) => quote!(.with_attr(stringify!(#ident), #ident)),
        });

        let (text_exprs, children, end_name) = match &self.finish {
            NxmlFinish::Closing {
                text_content,
                children,
                name,
            } => (&text_content[..], &children[..], name),
            _ => (&[][..], &[][..], name),
        };

        let mut static_text = String::new();
        let mut just_exprs = Vec::new();
        for expr_or_str in text_exprs {
            if !static_text.is_empty() {
                static_text.push(' ');
            }
            match expr_or_str {
                ExprOrStr::Str(lit) => static_text.push_str(&lit.value()),
                ExprOrStr::Expr(expr) => {
                    static_text.push_str("{}");
                    just_exprs.push(expr);
                }
            }
        }

        let text_content = if just_exprs.is_empty() {
            if !static_text.is_empty() {
                quote!(.with_text(#static_text))
            } else {
                quote!()
            }
        } else {
            quote!(.with_text(format!(#static_text, #(#just_exprs),*)))
        };

        let children = children.iter().map(|child| quote!(.with_child(#child)));
        tokens.append_all(quote! {{
            struct #name;
            // can use rust-analyzer rename action to change the tag in sync, and goto to jump between
            let _: #name = #end_name;

            nxml_rs::Element::new(stringify!(#name))
                #(#attrs)*
                #text_content
                #(#children)*
        }})
    }
}

#[proc_macro]
pub fn nxml(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as NxmlInput)
        .to_token_stream()
        .into()
}
