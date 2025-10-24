use crate::ast::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub level: DiagnosticLevel,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    entries: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push<S: Into<String>>(&mut self, message: S) {
        self.push_error_with_span(message, None);
    }

    pub fn push_with_location<S: Into<String>>(&mut self, message: S, line: usize, column: usize) {
        self.push_error_with_span(message, Some(SourceSpan::new(line, column, line, column)));
    }

    pub fn push_error_with_span<S: Into<String>>(&mut self, message: S, span: Option<SourceSpan>) {
        self.entries.push(Diagnostic {
            message: message.into(),
            level: DiagnosticLevel::Error,
            span,
        });
    }

    pub fn push_warning_with_span<S: Into<String>>(
        &mut self,
        message: S,
        span: Option<SourceSpan>,
    ) {
        self.entries.push(Diagnostic {
            message: message.into(),
            level: DiagnosticLevel::Warning,
            span,
        });
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.entries.extend(other.entries);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.entries
            .iter()
            .any(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
    }

    pub fn entries(&self) -> &[Diagnostic] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [Diagnostic] {
        &mut self.entries
    }
}
