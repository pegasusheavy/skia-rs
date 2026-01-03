//! SkSL (Skia Shading Language) parser.
//!
//! SkSL is a shading language similar to GLSL, designed for Skia's
//! runtime effects system. This module provides:
//! - Lexer for tokenizing SkSL source
//! - Parser for building an AST
//! - Type system for SkSL types
//! - Compilation to target languages (GLSL, SPIR-V, MSL, WGSL)


/// SkSL token types.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    /// Integer literal.
    IntLit(i32),
    /// Float literal.
    FloatLit(f32),
    /// Boolean literal.
    BoolLit(bool),
    /// Identifier.
    Ident(String),

    // Keywords
    /// `if`
    If,
    /// `else`
    Else,
    /// `for`
    For,
    /// `while`
    While,
    /// `do`
    Do,
    /// `return`
    Return,
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `discard`
    Discard,
    /// `struct`
    Struct,
    /// `uniform`
    Uniform,
    /// `in`
    In,
    /// `out`
    Out,
    /// `inout`
    InOut,
    /// `const`
    Const,
    /// `layout`
    Layout,

    // Types
    /// `void`
    Void,
    /// `bool`
    Bool,
    /// `int`
    Int,
    /// `float`
    Float,
    /// `half` (16-bit float)
    Half,
    /// `vec2`
    Vec2,
    /// `vec3`
    Vec3,
    /// `vec4`
    Vec4,
    /// `half2`
    Half2,
    /// `half3`
    Half3,
    /// `half4`
    Half4,
    /// `mat2`
    Mat2,
    /// `mat3`
    Mat3,
    /// `mat4`
    Mat4,
    /// `sampler2D`
    Sampler2D,
    /// `shader`
    Shader,
    /// `colorFilter`
    ColorFilter,
    /// `blender`
    Blender,

    // Operators
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `!`
    Bang,
    /// `&`
    And,
    /// `|`
    Or,
    /// `^`
    Caret,
    /// `~`
    Tilde,
    /// `<<`
    LtLt,
    /// `>>`
    GtGt,
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// `++`
    PlusPlus,
    /// `--`
    MinusMinus,
    /// `?`
    Question,
    /// `:`
    Colon,
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `;`
    Semi,

    // Delimiters
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,

    // Special
    /// End of file.
    Eof,
    /// Unknown token.
    Unknown(char),
}

/// SkSL lexer.
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            current_pos: 0,
        }
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let Some(&(pos, ch)) = self.chars.peek() else {
            return Token::Eof;
        };
        self.current_pos = pos;

        // Numbers
        if ch.is_ascii_digit() || (ch == '.' && self.peek_next_char().map_or(false, |c| c.is_ascii_digit())) {
            return self.scan_number();
        }

        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == '_' {
            return self.scan_identifier();
        }

        // Operators and delimiters
        self.chars.next();
        match ch {
            '+' => {
                if self.match_char('+') {
                    Token::PlusPlus
                } else if self.match_char('=') {
                    Token::PlusEq
                } else {
                    Token::Plus
                }
            }
            '-' => {
                if self.match_char('-') {
                    Token::MinusMinus
                } else if self.match_char('=') {
                    Token::MinusEq
                } else {
                    Token::Minus
                }
            }
            '*' => {
                if self.match_char('=') {
                    Token::StarEq
                } else {
                    Token::Star
                }
            }
            '/' => {
                if self.match_char('=') {
                    Token::SlashEq
                } else {
                    Token::Slash
                }
            }
            '%' => Token::Percent,
            '=' => {
                if self.match_char('=') {
                    Token::EqEq
                } else {
                    Token::Eq
                }
            }
            '!' => {
                if self.match_char('=') {
                    Token::NotEq
                } else {
                    Token::Bang
                }
            }
            '<' => {
                if self.match_char('=') {
                    Token::LtEq
                } else if self.match_char('<') {
                    Token::LtLt
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if self.match_char('=') {
                    Token::GtEq
                } else if self.match_char('>') {
                    Token::GtGt
                } else {
                    Token::Gt
                }
            }
            '&' => {
                if self.match_char('&') {
                    Token::AndAnd
                } else {
                    Token::And
                }
            }
            '|' => {
                if self.match_char('|') {
                    Token::OrOr
                } else {
                    Token::Or
                }
            }
            '^' => Token::Caret,
            '~' => Token::Tilde,
            '?' => Token::Question,
            ':' => Token::Colon,
            '.' => Token::Dot,
            ',' => Token::Comma,
            ';' => Token::Semi,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            _ => Token::Unknown(ch),
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while self.chars.peek().map_or(false, |&(_, c)| c.is_whitespace()) {
                self.chars.next();
            }

            // Check for comments
            if let Some(&(_, '/')) = self.chars.peek() {
                let mut chars_clone = self.chars.clone();
                chars_clone.next();

                if let Some(&(_, '/')) = chars_clone.peek() {
                    // Line comment
                    self.chars.next();
                    self.chars.next();
                    while self.chars.peek().map_or(false, |&(_, c)| c != '\n') {
                        self.chars.next();
                    }
                    continue;
                } else if let Some(&(_, '*')) = chars_clone.peek() {
                    // Block comment
                    self.chars.next();
                    self.chars.next();
                    while let Some(&(_, c)) = self.chars.peek() {
                        self.chars.next();
                        if c == '*' {
                            if self.chars.peek().map_or(false, |&(_, c)| c == '/') {
                                self.chars.next();
                                break;
                            }
                        }
                    }
                    continue;
                }
            }

            break;
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.chars.peek().map_or(false, |&(_, c)| c == expected) {
            self.chars.next();
            true
        } else {
            false
        }
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut chars_clone = self.chars.clone();
        chars_clone.next();
        chars_clone.peek().map(|&(_, c)| c)
    }

    fn scan_number(&mut self) -> Token {
        let start = self.current_pos;
        let mut has_dot = false;
        let mut has_exp = false;

        while let Some(&(pos, ch)) = self.chars.peek() {
            if ch.is_ascii_digit() {
                self.chars.next();
            } else if ch == '.' && !has_dot && !has_exp {
                has_dot = true;
                self.chars.next();
            } else if (ch == 'e' || ch == 'E') && !has_exp {
                has_exp = true;
                self.chars.next();
                if self.chars.peek().map_or(false, |&(_, c)| c == '+' || c == '-') {
                    self.chars.next();
                }
            } else if ch == 'f' || ch == 'F' {
                // Float suffix
                self.chars.next();
                break;
            } else {
                break;
            }
            self.current_pos = pos;
        }

        let end = self.chars.peek().map_or(self.source.len(), |&(pos, _)| pos);
        let text = &self.source[start..end].trim_end_matches(|c| c == 'f' || c == 'F');

        if has_dot || has_exp {
            Token::FloatLit(text.parse().unwrap_or(0.0))
        } else {
            Token::IntLit(text.parse().unwrap_or(0))
        }
    }

    fn scan_identifier(&mut self) -> Token {
        let start = self.current_pos;

        while self.chars.peek().map_or(false, |&(_, c)| c.is_ascii_alphanumeric() || c == '_') {
            self.chars.next();
        }

        let end = self.chars.peek().map_or(self.source.len(), |&(pos, _)| pos);
        let text = &self.source[start..end];

        // Check for keywords
        match text {
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "while" => Token::While,
            "do" => Token::Do,
            "return" => Token::Return,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "discard" => Token::Discard,
            "struct" => Token::Struct,
            "uniform" => Token::Uniform,
            "in" => Token::In,
            "out" => Token::Out,
            "inout" => Token::InOut,
            "const" => Token::Const,
            "layout" => Token::Layout,
            "void" => Token::Void,
            "bool" => Token::Bool,
            "int" => Token::Int,
            "float" => Token::Float,
            "half" => Token::Half,
            "vec2" | "float2" => Token::Vec2,
            "vec3" | "float3" => Token::Vec3,
            "vec4" | "float4" => Token::Vec4,
            "half2" => Token::Half2,
            "half3" => Token::Half3,
            "half4" => Token::Half4,
            "mat2" | "float2x2" => Token::Mat2,
            "mat3" | "float3x3" => Token::Mat3,
            "mat4" | "float4x4" => Token::Mat4,
            "sampler2D" => Token::Sampler2D,
            "shader" => Token::Shader,
            "colorFilter" => Token::ColorFilter,
            "blender" => Token::Blender,
            "true" => Token::BoolLit(true),
            "false" => Token::BoolLit(false),
            _ => Token::Ident(text.to_string()),
        }
    }
}

/// SkSL type.
#[derive(Debug, Clone, PartialEq)]
pub enum SkslType {
    /// Void type.
    Void,
    /// Boolean.
    Bool,
    /// 32-bit integer.
    Int,
    /// 32-bit float.
    Float,
    /// 16-bit float (half precision).
    Half,
    /// 2-component float vector.
    Vec2,
    /// 3-component float vector.
    Vec3,
    /// 4-component float vector.
    Vec4,
    /// 2-component half vector.
    Half2,
    /// 3-component half vector.
    Half3,
    /// 4-component half vector.
    Half4,
    /// 2x2 float matrix.
    Mat2,
    /// 3x3 float matrix.
    Mat3,
    /// 4x4 float matrix.
    Mat4,
    /// 2D texture sampler.
    Sampler2D,
    /// Shader child.
    Shader,
    /// Color filter child.
    ColorFilter,
    /// Blender child.
    Blender,
    /// Array type.
    Array(Box<SkslType>, usize),
    /// Struct type.
    Struct(String),
}

impl SkslType {
    /// Get the GLSL type name.
    pub fn glsl_name(&self) -> &'static str {
        match self {
            SkslType::Void => "void",
            SkslType::Bool => "bool",
            SkslType::Int => "int",
            SkslType::Float => "float",
            SkslType::Half => "float", // GLSL uses float for mediump
            SkslType::Vec2 => "vec2",
            SkslType::Vec3 => "vec3",
            SkslType::Vec4 => "vec4",
            SkslType::Half2 => "vec2",
            SkslType::Half3 => "vec3",
            SkslType::Half4 => "vec4",
            SkslType::Mat2 => "mat2",
            SkslType::Mat3 => "mat3",
            SkslType::Mat4 => "mat4",
            SkslType::Sampler2D => "sampler2D",
            SkslType::Shader => "sampler2D",
            SkslType::ColorFilter => "sampler2D",
            SkslType::Blender => "sampler2D",
            SkslType::Array(_, _) => "array",
            SkslType::Struct(_) => "struct",
        }
    }

    /// Get the WGSL type name.
    pub fn wgsl_name(&self) -> &'static str {
        match self {
            SkslType::Void => "()",
            SkslType::Bool => "bool",
            SkslType::Int => "i32",
            SkslType::Float => "f32",
            SkslType::Half => "f16",
            SkslType::Vec2 => "vec2<f32>",
            SkslType::Vec3 => "vec3<f32>",
            SkslType::Vec4 => "vec4<f32>",
            SkslType::Half2 => "vec2<f16>",
            SkslType::Half3 => "vec3<f16>",
            SkslType::Half4 => "vec4<f16>",
            SkslType::Mat2 => "mat2x2<f32>",
            SkslType::Mat3 => "mat3x3<f32>",
            SkslType::Mat4 => "mat4x4<f32>",
            SkslType::Sampler2D => "texture_2d<f32>",
            SkslType::Shader => "texture_2d<f32>",
            SkslType::ColorFilter => "texture_2d<f32>",
            SkslType::Blender => "texture_2d<f32>",
            SkslType::Array(_, _) => "array",
            SkslType::Struct(_) => "struct",
        }
    }

    /// Check if this is a scalar type.
    pub fn is_scalar(&self) -> bool {
        matches!(self, SkslType::Bool | SkslType::Int | SkslType::Float | SkslType::Half)
    }

    /// Check if this is a vector type.
    pub fn is_vector(&self) -> bool {
        matches!(
            self,
            SkslType::Vec2
                | SkslType::Vec3
                | SkslType::Vec4
                | SkslType::Half2
                | SkslType::Half3
                | SkslType::Half4
        )
    }

    /// Check if this is a matrix type.
    pub fn is_matrix(&self) -> bool {
        matches!(self, SkslType::Mat2 | SkslType::Mat3 | SkslType::Mat4)
    }

    /// Get the number of components for vector types.
    pub fn vector_size(&self) -> Option<usize> {
        match self {
            SkslType::Vec2 | SkslType::Half2 => Some(2),
            SkslType::Vec3 | SkslType::Half3 => Some(3),
            SkslType::Vec4 | SkslType::Half4 => Some(4),
            _ => None,
        }
    }
}

/// AST expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal.
    IntLit(i32),
    /// Float literal.
    FloatLit(f32),
    /// Boolean literal.
    BoolLit(bool),
    /// Variable reference.
    Var(String),
    /// Binary operation.
    Binary {
        /// Left operand.
        left: Box<Expr>,
        /// Operator.
        op: BinaryOp,
        /// Right operand.
        right: Box<Expr>,
    },
    /// Unary operation.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        expr: Box<Expr>,
    },
    /// Function call.
    Call {
        /// Function name.
        name: String,
        /// Arguments.
        args: Vec<Expr>,
    },
    /// Constructor (vec2, vec3, etc.).
    Constructor {
        /// Type being constructed.
        ty: SkslType,
        /// Arguments.
        args: Vec<Expr>,
    },
    /// Field access (e.g., v.x, v.rgb).
    Field {
        /// Base expression.
        expr: Box<Expr>,
        /// Field name.
        field: String,
    },
    /// Array index.
    Index {
        /// Base expression.
        expr: Box<Expr>,
        /// Index expression.
        index: Box<Expr>,
    },
    /// Ternary conditional.
    Ternary {
        /// Condition.
        cond: Box<Expr>,
        /// True branch.
        then_expr: Box<Expr>,
        /// False branch.
        else_expr: Box<Expr>,
    },
    /// Assignment.
    Assign {
        /// Target.
        target: Box<Expr>,
        /// Value.
        value: Box<Expr>,
    },
    /// Compound assignment (+=, -=, etc.).
    CompoundAssign {
        /// Target.
        target: Box<Expr>,
        /// Operator.
        op: BinaryOp,
        /// Value.
        value: Box<Expr>,
    },
    /// Post-increment/decrement.
    PostIncDec {
        /// Expression.
        expr: Box<Expr>,
        /// Increment (true) or decrement (false).
        inc: bool,
    },
    /// Pre-increment/decrement.
    PreIncDec {
        /// Expression.
        expr: Box<Expr>,
        /// Increment (true) or decrement (false).
        inc: bool,
    },
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Modulo.
    Mod,
    /// Equal.
    Eq,
    /// Not equal.
    NotEq,
    /// Less than.
    Lt,
    /// Less than or equal.
    LtEq,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    GtEq,
    /// Logical and.
    And,
    /// Logical or.
    Or,
    /// Bitwise and.
    BitAnd,
    /// Bitwise or.
    BitOr,
    /// Bitwise xor.
    BitXor,
    /// Left shift.
    Shl,
    /// Right shift.
    Shr,
}

impl BinaryOp {
    /// Get the GLSL operator string.
    pub fn glsl_str(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Negation.
    Neg,
    /// Logical not.
    Not,
    /// Bitwise not.
    BitNot,
}

impl UnaryOp {
    /// Get the GLSL operator string.
    pub fn glsl_str(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
        }
    }
}

/// AST statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Expression statement.
    Expr(Expr),
    /// Variable declaration.
    VarDecl {
        /// Variable type.
        ty: SkslType,
        /// Variable name.
        name: String,
        /// Initial value.
        init: Option<Expr>,
    },
    /// Block of statements.
    Block(Vec<Stmt>),
    /// If statement.
    If {
        /// Condition.
        cond: Expr,
        /// Then branch.
        then_branch: Box<Stmt>,
        /// Else branch.
        else_branch: Option<Box<Stmt>>,
    },
    /// For loop.
    For {
        /// Initialization.
        init: Option<Box<Stmt>>,
        /// Condition.
        cond: Option<Expr>,
        /// Update.
        update: Option<Expr>,
        /// Body.
        body: Box<Stmt>,
    },
    /// While loop.
    While {
        /// Condition.
        cond: Expr,
        /// Body.
        body: Box<Stmt>,
    },
    /// Do-while loop.
    DoWhile {
        /// Body.
        body: Box<Stmt>,
        /// Condition.
        cond: Expr,
    },
    /// Return statement.
    Return(Option<Expr>),
    /// Break statement.
    Break,
    /// Continue statement.
    Continue,
    /// Discard statement.
    Discard,
}

/// Function parameter.
#[derive(Debug, Clone)]
pub struct FnParam {
    /// Parameter type.
    pub ty: SkslType,
    /// Parameter name.
    pub name: String,
    /// Parameter qualifier.
    pub qualifier: ParamQualifier,
}

/// Parameter qualifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamQualifier {
    /// No qualifier (in).
    None,
    /// Input only.
    In,
    /// Output only.
    Out,
    /// Input and output.
    InOut,
}

/// Function declaration.
#[derive(Debug, Clone)]
pub struct FnDecl {
    /// Function name.
    pub name: String,
    /// Return type.
    pub return_type: SkslType,
    /// Parameters.
    pub params: Vec<FnParam>,
    /// Function body.
    pub body: Stmt,
}

/// Uniform declaration.
#[derive(Debug, Clone)]
pub struct UniformDecl {
    /// Uniform type.
    pub ty: SkslType,
    /// Uniform name.
    pub name: String,
    /// Array size (if array).
    pub array_size: Option<usize>,
}

/// Struct field.
#[derive(Debug, Clone)]
pub struct StructField {
    /// Field type.
    pub ty: SkslType,
    /// Field name.
    pub name: String,
}

/// Struct declaration.
#[derive(Debug, Clone)]
pub struct StructDecl {
    /// Struct name.
    pub name: String,
    /// Fields.
    pub fields: Vec<StructField>,
}

/// SkSL program (complete parsed shader).
#[derive(Debug, Clone, Default)]
pub struct SkslProgram {
    /// Struct declarations.
    pub structs: Vec<StructDecl>,
    /// Uniform declarations.
    pub uniforms: Vec<UniformDecl>,
    /// Function declarations.
    pub functions: Vec<FnDecl>,
    /// Child shader declarations.
    pub children: Vec<UniformDecl>,
}

/// SkSL parser.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    peeked: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn new(source: &'a str) -> Self {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            peeked: None,
        }
    }

    /// Parse a complete SkSL program.
    pub fn parse_program(&mut self) -> Result<SkslProgram, String> {
        let mut program = SkslProgram::default();

        while !self.check(&Token::Eof) {
            if self.check(&Token::Struct) {
                program.structs.push(self.parse_struct()?);
            } else if self.check(&Token::Uniform) {
                program.uniforms.push(self.parse_uniform()?);
            } else if self.is_type_token() {
                // Function declaration
                program.functions.push(self.parse_function()?);
            } else {
                return Err(format!("Unexpected token: {:?}", self.current));
            }
        }

        Ok(program)
    }

    fn advance(&mut self) -> Token {
        let current = std::mem::replace(
            &mut self.current,
            self.peeked.take().unwrap_or_else(|| self.lexer.next_token()),
        );
        current
    }

    fn peek(&mut self) -> &Token {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token());
        }
        self.peeked.as_ref().unwrap()
    }

    fn check(&self, expected: &Token) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(expected)
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, String> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.current))
        }
    }

    fn is_type_token(&self) -> bool {
        matches!(
            self.current,
            Token::Void
                | Token::Bool
                | Token::Int
                | Token::Float
                | Token::Half
                | Token::Vec2
                | Token::Vec3
                | Token::Vec4
                | Token::Half2
                | Token::Half3
                | Token::Half4
                | Token::Mat2
                | Token::Mat3
                | Token::Mat4
                | Token::Sampler2D
                | Token::Shader
                | Token::ColorFilter
                | Token::Blender
        )
    }

    fn parse_type(&mut self) -> Result<SkslType, String> {
        let ty = match &self.current {
            Token::Void => SkslType::Void,
            Token::Bool => SkslType::Bool,
            Token::Int => SkslType::Int,
            Token::Float => SkslType::Float,
            Token::Half => SkslType::Half,
            Token::Vec2 => SkslType::Vec2,
            Token::Vec3 => SkslType::Vec3,
            Token::Vec4 => SkslType::Vec4,
            Token::Half2 => SkslType::Half2,
            Token::Half3 => SkslType::Half3,
            Token::Half4 => SkslType::Half4,
            Token::Mat2 => SkslType::Mat2,
            Token::Mat3 => SkslType::Mat3,
            Token::Mat4 => SkslType::Mat4,
            Token::Sampler2D => SkslType::Sampler2D,
            Token::Shader => SkslType::Shader,
            Token::ColorFilter => SkslType::ColorFilter,
            Token::Blender => SkslType::Blender,
            Token::Ident(name) => SkslType::Struct(name.clone()),
            _ => return Err(format!("Expected type, got {:?}", self.current)),
        };
        self.advance();
        Ok(ty)
    }

    fn parse_struct(&mut self) -> Result<StructDecl, String> {
        self.expect(&Token::Struct)?;
        let name = match self.advance() {
            Token::Ident(name) => name,
            t => return Err(format!("Expected struct name, got {:?}", t)),
        };
        self.expect(&Token::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) {
            let ty = self.parse_type()?;
            let field_name = match self.advance() {
                Token::Ident(name) => name,
                t => return Err(format!("Expected field name, got {:?}", t)),
            };
            self.expect(&Token::Semi)?;
            fields.push(StructField { ty, name: field_name });
        }

        self.expect(&Token::RBrace)?;
        self.expect(&Token::Semi)?;

        Ok(StructDecl { name, fields })
    }

    fn parse_uniform(&mut self) -> Result<UniformDecl, String> {
        self.expect(&Token::Uniform)?;
        let ty = self.parse_type()?;
        let name = match self.advance() {
            Token::Ident(name) => name,
            t => return Err(format!("Expected uniform name, got {:?}", t)),
        };

        let array_size = if self.check(&Token::LBracket) {
            self.advance();
            let size = match self.advance() {
                Token::IntLit(n) => n as usize,
                t => return Err(format!("Expected array size, got {:?}", t)),
            };
            self.expect(&Token::RBracket)?;
            Some(size)
        } else {
            None
        };

        self.expect(&Token::Semi)?;

        Ok(UniformDecl { ty, name, array_size })
    }

    fn parse_function(&mut self) -> Result<FnDecl, String> {
        let return_type = self.parse_type()?;
        let name = match self.advance() {
            Token::Ident(name) => name,
            t => return Err(format!("Expected function name, got {:?}", t)),
        };

        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        while !self.check(&Token::RParen) {
            let qualifier = if self.check(&Token::In) {
                self.advance();
                ParamQualifier::In
            } else if self.check(&Token::Out) {
                self.advance();
                ParamQualifier::Out
            } else if self.check(&Token::InOut) {
                self.advance();
                ParamQualifier::InOut
            } else {
                ParamQualifier::None
            };

            let ty = self.parse_type()?;
            let param_name = match self.advance() {
                Token::Ident(name) => name,
                t => return Err(format!("Expected parameter name, got {:?}", t)),
            };

            params.push(FnParam {
                ty,
                name: param_name,
                qualifier,
            });

            if !self.check(&Token::RParen) {
                self.expect(&Token::Comma)?;
            }
        }

        self.expect(&Token::RParen)?;
        let body = self.parse_block()?;

        Ok(FnDecl {
            name,
            return_type,
            params,
            body,
        })
    }

    fn parse_block(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::LBrace)?;
        let mut stmts = Vec::new();

        while !self.check(&Token::RBrace) {
            stmts.push(self.parse_statement()?);
        }

        self.expect(&Token::RBrace)?;
        Ok(Stmt::Block(stmts))
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        if self.check(&Token::LBrace) {
            return self.parse_block();
        }

        if self.check(&Token::If) {
            return self.parse_if();
        }

        if self.check(&Token::For) {
            return self.parse_for();
        }

        if self.check(&Token::While) {
            return self.parse_while();
        }

        if self.check(&Token::Do) {
            return self.parse_do_while();
        }

        if self.check(&Token::Return) {
            self.advance();
            let expr = if !self.check(&Token::Semi) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(&Token::Semi)?;
            return Ok(Stmt::Return(expr));
        }

        if self.check(&Token::Break) {
            self.advance();
            self.expect(&Token::Semi)?;
            return Ok(Stmt::Break);
        }

        if self.check(&Token::Continue) {
            self.advance();
            self.expect(&Token::Semi)?;
            return Ok(Stmt::Continue);
        }

        if self.check(&Token::Discard) {
            self.advance();
            self.expect(&Token::Semi)?;
            return Ok(Stmt::Discard);
        }

        // Check for variable declaration
        if self.is_type_token() || self.check(&Token::Const) {
            let _const_qual = if self.check(&Token::Const) {
                self.advance();
                true
            } else {
                false
            };

            let ty = self.parse_type()?;
            let name = match self.advance() {
                Token::Ident(name) => name,
                t => return Err(format!("Expected variable name, got {:?}", t)),
            };

            let init = if self.check(&Token::Eq) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            self.expect(&Token::Semi)?;
            return Ok(Stmt::VarDecl { ty, name, init });
        }

        // Expression statement
        let expr = self.parse_expression()?;
        self.expect(&Token::Semi)?;
        Ok(Stmt::Expr(expr))
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::If)?;
        self.expect(&Token::LParen)?;
        let cond = self.parse_expression()?;
        self.expect(&Token::RParen)?;
        let then_branch = Box::new(self.parse_statement()?);

        let else_branch = if self.check(&Token::Else) {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::For)?;
        self.expect(&Token::LParen)?;

        let init = if !self.check(&Token::Semi) {
            Some(Box::new(self.parse_statement()?))
        } else {
            self.advance();
            None
        };

        let cond = if !self.check(&Token::Semi) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&Token::Semi)?;

        let update = if !self.check(&Token::RParen) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&Token::RParen)?;

        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::For {
            init,
            cond,
            update,
            body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::While)?;
        self.expect(&Token::LParen)?;
        let cond = self.parse_expression()?;
        self.expect(&Token::RParen)?;
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::While { cond, body })
    }

    fn parse_do_while(&mut self) -> Result<Stmt, String> {
        self.expect(&Token::Do)?;
        let body = Box::new(self.parse_statement()?);
        self.expect(&Token::While)?;
        self.expect(&Token::LParen)?;
        let cond = self.parse_expression()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::Semi)?;

        Ok(Stmt::DoWhile { body, cond })
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let expr = self.parse_ternary()?;

        if self.check(&Token::Eq) {
            self.advance();
            let value = self.parse_assignment()?;
            return Ok(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
            });
        }

        if self.check(&Token::PlusEq)
            || self.check(&Token::MinusEq)
            || self.check(&Token::StarEq)
            || self.check(&Token::SlashEq)
        {
            let op = match self.advance() {
                Token::PlusEq => BinaryOp::Add,
                Token::MinusEq => BinaryOp::Sub,
                Token::StarEq => BinaryOp::Mul,
                Token::SlashEq => BinaryOp::Div,
                _ => unreachable!(),
            };
            let value = self.parse_assignment()?;
            return Ok(Expr::CompoundAssign {
                target: Box::new(expr),
                op,
                value: Box::new(value),
            });
        }

        Ok(expr)
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let cond = self.parse_or()?;

        if self.check(&Token::Question) {
            self.advance();
            let then_expr = self.parse_expression()?;
            self.expect(&Token::Colon)?;
            let else_expr = self.parse_ternary()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            });
        }

        Ok(cond)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;

        while self.check(&Token::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;

        while self.check(&Token::AndAnd) {
            self.advance();
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_comparison()?;

        while self.check(&Token::EqEq) || self.check(&Token::NotEq) {
            let op = match self.advance() {
                Token::EqEq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::NotEq,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_additive()?;

        while self.check(&Token::Lt)
            || self.check(&Token::LtEq)
            || self.check(&Token::Gt)
            || self.check(&Token::GtEq)
        {
            let op = match self.advance() {
                Token::Lt => BinaryOp::Lt,
                Token::LtEq => BinaryOp::LtEq,
                Token::Gt => BinaryOp::Gt,
                Token::GtEq => BinaryOp::GtEq,
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_multiplicative()?;

        while self.check(&Token::Plus) || self.check(&Token::Minus) {
            let op = match self.advance() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;

        while self.check(&Token::Star) || self.check(&Token::Slash) || self.check(&Token::Percent) {
            let op = match self.advance() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.check(&Token::Minus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }

        if self.check(&Token::Bang) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }

        if self.check(&Token::Tilde) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(expr),
            });
        }

        if self.check(&Token::PlusPlus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::PreIncDec {
                expr: Box::new(expr),
                inc: true,
            });
        }

        if self.check(&Token::MinusMinus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::PreIncDec {
                expr: Box::new(expr),
                inc: false,
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&Token::Dot) {
                self.advance();
                let field = match self.advance() {
                    Token::Ident(name) => name,
                    t => return Err(format!("Expected field name, got {:?}", t)),
                };
                expr = Expr::Field {
                    expr: Box::new(expr),
                    field,
                };
            } else if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(&Token::RBracket)?;
                expr = Expr::Index {
                    expr: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.check(&Token::LParen) {
                // Function call
                if let Expr::Var(name) = expr {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&Token::RParen) {
                        args.push(self.parse_expression()?);
                        if !self.check(&Token::RParen) {
                            self.expect(&Token::Comma)?;
                        }
                    }
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call { name, args };
                } else {
                    break;
                }
            } else if self.check(&Token::PlusPlus) {
                self.advance();
                expr = Expr::PostIncDec {
                    expr: Box::new(expr),
                    inc: true,
                };
            } else if self.check(&Token::MinusMinus) {
                self.advance();
                expr = Expr::PostIncDec {
                    expr: Box::new(expr),
                    inc: false,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match &self.current {
            Token::IntLit(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Token::FloatLit(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::FloatLit(n))
            }
            Token::BoolLit(b) => {
                let b = *b;
                self.advance();
                Ok(Expr::BoolLit(b))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Var(name))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            // Type constructors
            Token::Vec2
            | Token::Vec3
            | Token::Vec4
            | Token::Half2
            | Token::Half3
            | Token::Half4
            | Token::Mat2
            | Token::Mat3
            | Token::Mat4
            | Token::Float
            | Token::Int
            | Token::Bool => {
                let ty = self.parse_type()?;
                self.expect(&Token::LParen)?;
                let mut args = Vec::new();
                while !self.check(&Token::RParen) {
                    args.push(self.parse_expression()?);
                    if !self.check(&Token::RParen) {
                        self.expect(&Token::Comma)?;
                    }
                }
                self.expect(&Token::RParen)?;
                Ok(Expr::Constructor { ty, args })
            }
            _ => Err(format!("Unexpected token in expression: {:?}", self.current)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let mut lexer = Lexer::new("float x = 1.0;");
        assert!(matches!(lexer.next_token(), Token::Float));
        assert!(matches!(lexer.next_token(), Token::Ident(s) if s == "x"));
        assert!(matches!(lexer.next_token(), Token::Eq));
        assert!(matches!(lexer.next_token(), Token::FloatLit(n) if (n - 1.0).abs() < 0.01));
        assert!(matches!(lexer.next_token(), Token::Semi));
        assert!(matches!(lexer.next_token(), Token::Eof));
    }

    #[test]
    fn test_lexer_operators() {
        let mut lexer = Lexer::new("+ - * / == != <= >= && ||");
        assert!(matches!(lexer.next_token(), Token::Plus));
        assert!(matches!(lexer.next_token(), Token::Minus));
        assert!(matches!(lexer.next_token(), Token::Star));
        assert!(matches!(lexer.next_token(), Token::Slash));
        assert!(matches!(lexer.next_token(), Token::EqEq));
        assert!(matches!(lexer.next_token(), Token::NotEq));
        assert!(matches!(lexer.next_token(), Token::LtEq));
        assert!(matches!(lexer.next_token(), Token::GtEq));
        assert!(matches!(lexer.next_token(), Token::AndAnd));
        assert!(matches!(lexer.next_token(), Token::OrOr));
    }

    #[test]
    fn test_lexer_comments() {
        let mut lexer = Lexer::new("x // comment\ny /* block */ z");
        assert!(matches!(lexer.next_token(), Token::Ident(s) if s == "x"));
        assert!(matches!(lexer.next_token(), Token::Ident(s) if s == "y"));
        assert!(matches!(lexer.next_token(), Token::Ident(s) if s == "z"));
    }

    #[test]
    fn test_parser_simple_function() {
        let source = r#"
            vec4 main(vec2 fragCoord) {
                return vec4(1.0, 0.0, 0.0, 1.0);
            }
        "#;
        let mut parser = Parser::new(source);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
        assert_eq!(program.functions[0].return_type, SkslType::Vec4);
    }

    #[test]
    fn test_parser_uniforms() {
        let source = r#"
            uniform float time;
            uniform vec2 resolution;
            vec4 main(vec2 coord) {
                return vec4(0.0);
            }
        "#;
        let mut parser = Parser::new(source);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.uniforms.len(), 2);
        assert_eq!(program.uniforms[0].name, "time");
        assert_eq!(program.uniforms[1].name, "resolution");
    }
}
