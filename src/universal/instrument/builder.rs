use crate::universal::instrument::Instrument;
use crate::universal::instrument::kind::InstrumentKind;
use crate::universal::symbol::Symbol;

pub struct InstrumentBuilder {
    base: Option<Symbol>,
    quote: Option<Symbol>,
    kind: Option<InstrumentKind>,
}

impl InstrumentBuilder {
    pub fn new() -> Self {
        InstrumentBuilder {
            base: None,
            quote: None,
            kind: None,
        }
    }

    pub fn base<S>(mut self, base: S) -> Self
                   where
                       S: Into<Symbol>,
    {
        self.base = Some(base.into());
        self
    }

    pub fn quote<S>(mut self, quote: S) -> Self
                    where
                        S: Into<Symbol>,
    {
        self.quote = Some(quote.into());
        self
    }

    pub fn kind(mut self, kind: InstrumentKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn build(self) -> Result<Instrument, &'static str> {
        Ok(Instrument {
            base: self.base.ok_or("Base is missing")?,
            quote: self.quote.ok_or("Quote is missing")?,
            kind: self.kind.ok_or("Instrument kind is missing")?,
        })
    }
}