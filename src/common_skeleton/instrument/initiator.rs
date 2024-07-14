use crate::common_skeleton::{
    instrument::{kind::InstrumentKind, Instrument},
    token::Token,
};

pub struct InstrumentInitiator
{
    base: Option<Token>,
    quote: Option<Token>,
    kind: Option<InstrumentKind>,
}

impl InstrumentInitiator
{
    pub fn new() -> Self
    {
        InstrumentInitiator { base: None,
                              quote: None,
                              kind: None }
    }

    pub fn base<S>(mut self, base: S) -> Self
        where S: Into<Token>
    {
        self.base = Some(base.into());
        self
    }

    pub fn quote<S>(mut self, quote: S) -> Self
        where S: Into<Token>
    {
        self.quote = Some(quote.into());
        self
    }

    pub fn kind(mut self, kind: InstrumentKind) -> Self
    {
        self.kind = Some(kind);
        self
    }

    pub fn initiate(self) -> Result<Instrument, &'static str>
    {
        Ok(Instrument { base: self.base.ok_or("Base is missing")?,
                        quote: self.quote.ok_or("Quote is missing")?,
                        kind: self.kind.ok_or("Instrument kind is missing")? })
    }
}
