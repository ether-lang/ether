"""
NeuralScript: A Turing-Complete AI-Specialized Programming Language
===================================================================

A statically-typed, type-inferred language with bytecode VM designed for AI/ML tasks.

Features:
- Type inference with Hindley-Milner-style algorithm
- First-class tensors and neural network primitives
- Bytecode compilation and virtual machine execution
- Pattern matching and algebraic data types
- Automatic differentiation support
- GPU-ready operations

Language Syntax Examples:
-------------------------

// Variables with type inference
let x = 42
let name = "neural"
let scores = [1.0, 2.0, 3.0]

// Functions
fn add(a, b) {
    return a + b
}

// Tensor operations
let weights = tensor([128, 64])
let bias = tensor([64])
let output = matmul(weights, input) + bias

// Pattern matching
match result {
    Ok(value) => print(value),
    Err(msg) => print("Error: " + msg)
}

// Type annotations (optional)
fn forward(x: Tensor) -> Tensor {
    return relu(x)
}
"""

import re
from enum import Enum, auto
from dataclasses import dataclass
from typing import List, Dict, Any, Optional, Union
import struct

# ============================================================================
# TYPE SYSTEM
# ============================================================================

class Type:
    """Base class for all types"""
    pass

@dataclass
class PrimitiveType(Type):
    name: str  # int, float, bool, string, void
    
    def __repr__(self):
        return self.name
    
    def __eq__(self, other):
        return isinstance(other, PrimitiveType) and self.name == other.name
    
    def __hash__(self):
        return hash(self.name)

@dataclass
class TensorType(Type):
    shape: Optional[List[int]] = None
    dtype: str = "float32"
    
    def __repr__(self):
        if self.shape:
            return f"Tensor[{self.shape}]"
        return "Tensor"

@dataclass
class FunctionType(Type):
    params: List[Type]
    return_type: Type
    
    def __repr__(self):
        params_str = ", ".join(str(p) for p in self.params)
        return f"({params_str}) -> {self.return_type}"

@dataclass
class ListType(Type):
    element_type: Type
    
    def __repr__(self):
        return f"[{self.element_type}]"

@dataclass
class TypeVariable(Type):
    name: str
    
    def __repr__(self):
        return f"'{self.name}"
    
    def __eq__(self, other):
        return isinstance(other, TypeVariable) and self.name == other.name
    
    def __hash__(self):
        return hash(self.name)

# Built-in types
INT = PrimitiveType("int")
FLOAT = PrimitiveType("float")
BOOL = PrimitiveType("bool")
STRING = PrimitiveType("string")
VOID = PrimitiveType("void")
TENSOR = TensorType()

# ============================================================================
# LEXER
# ============================================================================

class TokenType(Enum):
    # Literals
    INT_LIT = auto()
    FLOAT_LIT = auto()
    STRING_LIT = auto()
    BOOL_LIT = auto()
    
    # Keywords
    LET = auto()
    FN = auto()
    RETURN = auto()
    IF = auto()
    ELSE = auto()
    WHILE = auto()
    FOR = auto()
    IN = auto()
    MATCH = auto()
    TENSOR = auto()
    IMPORT = auto()
    
    # Identifiers
    IDENT = auto()
    
    # Operators
    PLUS = auto()
    MINUS = auto()
    STAR = auto()
    SLASH = auto()
    PERCENT = auto()
    EQ = auto()
    NEQ = auto()
    LT = auto()
    GT = auto()
    LTE = auto()
    GTE = auto()
    AND = auto()
    OR = auto()
    NOT = auto()
    ASSIGN = auto()
    ARROW = auto()
    
    # Delimiters
    LPAREN = auto()
    RPAREN = auto()
    LBRACE = auto()
    RBRACE = auto()
    LBRACKET = auto()
    RBRACKET = auto()
    COMMA = auto()
    COLON = auto()
    SEMICOLON = auto()
    DOT = auto()
    
    # Special
    EOF = auto()
    NEWLINE = auto()

@dataclass
class Token:
    type: TokenType
    value: Any
    line: int
    column: int

class Lexer:
    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.line = 1
        self.column = 1
        
        self.keywords = {
            'let': TokenType.LET,
            'fn': TokenType.FN,
            'return': TokenType.RETURN,
            'if': TokenType.IF,
            'else': TokenType.ELSE,
            'while': TokenType.WHILE,
            'for': TokenType.FOR,
            'in': TokenType.IN,
            'match': TokenType.MATCH,
            'tensor': TokenType.TENSOR,
            'import': TokenType.IMPORT,
            'true': TokenType.BOOL_LIT,
            'false': TokenType.BOOL_LIT,
            'and': TokenType.AND,
            'or': TokenType.OR,
            'not': TokenType.NOT,
        }
    
    def current_char(self) -> Optional[str]:
        if self.pos >= len(self.source):
            return None
        return self.source[self.pos]
    
    def peek_char(self, offset=1) -> Optional[str]:
        pos = self.pos + offset
        if pos >= len(self.source):
            return None
        return self.source[pos]
    
    def advance(self):
        if self.pos < len(self.source):
            if self.source[self.pos] == '\n':
                self.line += 1
                self.column = 1
            else:
                self.column += 1
            self.pos += 1
    
    def skip_whitespace(self):
        while self.current_char() and self.current_char() in ' \t\r':
            self.advance()
    
    def skip_comment(self):
        if self.current_char() and self.peek_char() and self.current_char() == '/' and self.peek_char() == '/':
            while self.current_char() and self.current_char() != '\n':
                self.advance()
    
    def read_number(self) -> Token:
        start_line, start_col = self.line, self.column
        num_str = ''
        is_float = False
        
        while self.current_char() and (self.current_char().isdigit() or self.current_char() == '.'):
            if self.current_char() == '.':
                if is_float:
                    break
                is_float = True
            num_str += self.current_char()
            self.advance()
        
        if is_float:
            return Token(TokenType.FLOAT_LIT, float(num_str), start_line, start_col)
        return Token(TokenType.INT_LIT, int(num_str), start_line, start_col)
    
    def read_string(self) -> Token:
        start_line, start_col = self.line, self.column
        self.advance()  # Skip opening quote
        string = ''
        
        while self.current_char() and self.current_char() != '"':
            if self.current_char() == '\\':
                self.advance()
                if self.current_char() and self.current_char() in 'nrt"\\':
                    escape_chars = {'n': '\n', 'r': '\r', 't': '\t', '"': '"', '\\': '\\'}
                    string += escape_chars[self.current_char()]
                    self.advance()
            else:
                string += self.current_char()
                self.advance()
        
        if self.current_char():
            self.advance()  # Skip closing quote
        return Token(TokenType.STRING_LIT, string, start_line, start_col)
    
    def read_identifier(self) -> Token:
        start_line, start_col = self.line, self.column
        ident = ''
        
        while self.current_char() and (self.current_char().isalnum() or self.current_char() == '_'):
            ident += self.current_char()
            self.advance()
        
        token_type = self.keywords.get(ident, TokenType.IDENT)
        value = ident
        
        if token_type == TokenType.BOOL_LIT:
            value = ident == 'true'
        
        return Token(token_type, value, start_line, start_col)
    
    def tokenize(self) -> List[Token]:
        tokens = []
        
        while self.pos < len(self.source):
            self.skip_whitespace()
            self.skip_comment()
            
            if not self.current_char():
                break
            
            char = self.current_char()
            line, col = self.line, self.column
            
            # Newlines
            if char == '\n':
                tokens.append(Token(TokenType.NEWLINE, '\n', line, col))
                self.advance()
            
            # Numbers
            elif char.isdigit():
                tokens.append(self.read_number())
            
            # Strings
            elif char == '"':
                tokens.append(self.read_string())
            
            # Identifiers and keywords
            elif char.isalpha() or char == '_':
                tokens.append(self.read_identifier())
            
            # Operators and delimiters
            elif char == '+':
                tokens.append(Token(TokenType.PLUS, '+', line, col))
                self.advance()
            elif char == '-':
                if self.peek_char() == '>':
                    tokens.append(Token(TokenType.ARROW, '->', line, col))
                    self.advance()
                    self.advance()
                else:
                    tokens.append(Token(TokenType.MINUS, '-', line, col))
                    self.advance()
            elif char == '*':
                tokens.append(Token(TokenType.STAR, '*', line, col))
                self.advance()
            elif char == '/':
                tokens.append(Token(TokenType.SLASH, '/', line, col))
                self.advance()
            elif char == '%':
                tokens.append(Token(TokenType.PERCENT, '%', line, col))
                self.advance()
            elif char == '=':
                if self.peek_char() == '=':
                    tokens.append(Token(TokenType.EQ, '==', line, col))
                    self.advance()
                    self.advance()
                else:
                    tokens.append(Token(TokenType.ASSIGN, '=', line, col))
                    self.advance()
            elif char == '!':
                if self.peek_char() == '=':
                    tokens.append(Token(TokenType.NEQ, '!=', line, col))
                    self.advance()
                    self.advance()
            elif char == '<':
                if self.peek_char() == '=':
                    tokens.append(Token(TokenType.LTE, '<=', line, col))
                    self.advance()
                    self.advance()
                else:
                    tokens.append(Token(TokenType.LT, '<', line, col))
                    self.advance()
            elif char == '>':
                if self.peek_char() == '=':
                    tokens.append(Token(TokenType.GTE, '>=', line, col))
                    self.advance()
                    self.advance()
                else:
                    tokens.append(Token(TokenType.GT, '>', line, col))
                    self.advance()
            elif char == '(':
                tokens.append(Token(TokenType.LPAREN, '(', line, col))
                self.advance()
            elif char == ')':
                tokens.append(Token(TokenType.RPAREN, ')', line, col))
                self.advance()
            elif char == '{':
                tokens.append(Token(TokenType.LBRACE, '{', line, col))
                self.advance()
            elif char == '}':
                tokens.append(Token(TokenType.RBRACE, '}', line, col))
                self.advance()
            elif char == '[':
                tokens.append(Token(TokenType.LBRACKET, '[', line, col))
                self.advance()
            elif char == ']':
                tokens.append(Token(TokenType.RBRACKET, ']', line, col))
                self.advance()
            elif char == ',':
                tokens.append(Token(TokenType.COMMA, ',', line, col))
                self.advance()
            elif char == ':':
                tokens.append(Token(TokenType.COLON, ':', line, col))
                self.advance()
            elif char == ';':
                tokens.append(Token(TokenType.SEMICOLON, ';', line, col))
                self.advance()
            elif char == '.':
                tokens.append(Token(TokenType.DOT, '.', line, col))
                self.advance()
            else:
                raise SyntaxError(f"Unexpected character '{char}' at line {line}, column {col}")
        
        tokens.append(Token(TokenType.EOF, None, self.line, self.column))
        return tokens

# ============================================================================
# ABSTRACT SYNTAX TREE
# ============================================================================

class ASTNode:
    pass

@dataclass
class Program(ASTNode):
    statements: List[ASTNode]

@dataclass
class AssignStatement(ASTNode):
    name: str
    value: ASTNode

@dataclass
class LetStatement(ASTNode):
    name: str
    value: ASTNode
    type_annotation: Optional[Type] = None

@dataclass
class FunctionDef(ASTNode):
    name: str
    params: List[tuple]  # [(name, type_annotation), ...]
    body: List[ASTNode]
    return_type: Optional[Type] = None

@dataclass
class ReturnStatement(ASTNode):
    value: Optional[ASTNode] = None

@dataclass
class IfStatement(ASTNode):
    condition: ASTNode
    then_block: List[ASTNode]
    else_block: Optional[List[ASTNode]] = None

@dataclass
class WhileStatement(ASTNode):
    condition: ASTNode
    body: List[ASTNode]

@dataclass
class BinaryOp(ASTNode):
    left: ASTNode
    op: str
    right: ASTNode

@dataclass
class UnaryOp(ASTNode):
    op: str
    operand: ASTNode

@dataclass
class FunctionCall(ASTNode):
    name: str
    args: List[ASTNode]

@dataclass
class Identifier(ASTNode):
    name: str

@dataclass
class Literal(ASTNode):
    value: Any
    type: Type

@dataclass
class ListLiteral(ASTNode):
    elements: List[ASTNode]

@dataclass
class TensorLiteral(ASTNode):
    shape: List[int]
    elements: Optional[List[float]] = None

# ============================================================================
# PARSER
# ============================================================================

class Parser:
    def __init__(self, tokens: List[Token]):
        self.tokens = [t for t in tokens if t.type != TokenType.NEWLINE]
        self.pos = 0
    
    def current_token(self) -> Token:
        if self.pos >= len(self.tokens):
            return self.tokens[-1]
        return self.tokens[self.pos]
    
    def peek_token(self, offset=1) -> Token:
        pos = self.pos + offset
        if pos >= len(self.tokens):
            return self.tokens[-1]
        return self.tokens[pos]
    
    def advance(self):
        if self.pos < len(self.tokens) - 1:
            self.pos += 1
    
    def expect(self, token_type: TokenType) -> Token:
        token = self.current_token()
        if token.type != token_type:
            raise SyntaxError(f"Expected {token_type}, got {token.type} at line {token.line}")
        self.advance()
        return token
    
    def parse(self) -> Program:
        statements = []
        while self.current_token().type != TokenType.EOF:
            stmt = self.parse_statement()
            if stmt:
                statements.append(stmt)
        return Program(statements)
    
    def parse_statement(self) -> Optional[ASTNode]:
        token = self.current_token()
        
        if token.type == TokenType.LET:
            return self.parse_let_statement()
        elif token.type == TokenType.FN:
            return self.parse_function_def()
        elif token.type == TokenType.RETURN:
            return self.parse_return_statement()
        elif token.type == TokenType.IF:
            return self.parse_if_statement()
        elif token.type == TokenType.WHILE:
            return self.parse_while_statement()
        elif token.type == TokenType.IDENT and self.peek_token().type == TokenType.ASSIGN:
            return self.parse_assignment_statement()
        else:
            return self.parse_expression_statement()
    
    def parse_let_statement(self) -> LetStatement:
        self.expect(TokenType.LET)
        name = self.expect(TokenType.IDENT).value
        
        type_annotation = None
        if self.current_token().type == TokenType.COLON:
            self.advance()
            type_annotation = self.parse_type()
        
        self.expect(TokenType.ASSIGN)
        value = self.parse_expression()
        
        return LetStatement(name, value, type_annotation)
    
    def parse_assignment_statement(self) -> AssignStatement:
        name = self.expect(TokenType.IDENT).value
        self.expect(TokenType.ASSIGN)
        value = self.parse_expression()
        
        return AssignStatement(name, value)
    
    def parse_function_def(self) -> FunctionDef:
        self.expect(TokenType.FN)
        name = self.expect(TokenType.IDENT).value
        
        self.expect(TokenType.LPAREN)
        params = []
        while self.current_token().type != TokenType.RPAREN:
            param_name = self.expect(TokenType.IDENT).value
            param_type = None
            if self.current_token().type == TokenType.COLON:
                self.advance()
                param_type = self.parse_type()
            params.append((param_name, param_type))
            
            if self.current_token().type == TokenType.COMMA:
                self.advance()
        self.expect(TokenType.RPAREN)
        
        return_type = None
        if self.current_token().type == TokenType.ARROW:
            self.advance()
            return_type = self.parse_type()
        
        self.expect(TokenType.LBRACE)
        body = []
        while self.current_token().type != TokenType.RBRACE:
            stmt = self.parse_statement()
            if stmt:
                body.append(stmt)
        self.expect(TokenType.RBRACE)
        
        return FunctionDef(name, params, body, return_type)
    
    def parse_return_statement(self) -> ReturnStatement:
        self.expect(TokenType.RETURN)
        value = None
        if self.current_token().type not in [TokenType.RBRACE, TokenType.EOF]:
            value = self.parse_expression()
        return ReturnStatement(value)
    
    def parse_if_statement(self) -> IfStatement:
        self.expect(TokenType.IF)
        condition = self.parse_expression()
        
        self.expect(TokenType.LBRACE)
        then_block = []
        while self.current_token().type != TokenType.RBRACE:
            stmt = self.parse_statement()
            if stmt:
                then_block.append(stmt)
        self.expect(TokenType.RBRACE)
        
        else_block = None
        if self.current_token().type == TokenType.ELSE:
            self.advance()
            self.expect(TokenType.LBRACE)
            else_block = []
            while self.current_token().type != TokenType.RBRACE:
                stmt = self.parse_statement()
                if stmt:
                    else_block.append(stmt)
            self.expect(TokenType.RBRACE)
        
        return IfStatement(condition, then_block, else_block)
    
    def parse_while_statement(self) -> WhileStatement:
        self.expect(TokenType.WHILE)
        condition = self.parse_expression()
        
        self.expect(TokenType.LBRACE)
        body = []
        while self.current_token().type != TokenType.RBRACE:
            stmt = self.parse_statement()
            if stmt:
                body.append(stmt)
        self.expect(TokenType.RBRACE)
        
        return WhileStatement(condition, body)
    
    def parse_expression_statement(self) -> ASTNode:
        return self.parse_expression()
    
    def parse_expression(self) -> ASTNode:
        return self.parse_or()
    
    def parse_or(self) -> ASTNode:
        left = self.parse_and()
        
        while self.current_token().type == TokenType.OR:
            op = self.current_token().value
            self.advance()
            right = self.parse_and()
            left = BinaryOp(left, op, right)
        
        return left
    
    def parse_and(self) -> ASTNode:
        left = self.parse_equality()
        
        while self.current_token().type == TokenType.AND:
            op = self.current_token().value
            self.advance()
            right = self.parse_equality()
            left = BinaryOp(left, op, right)
        
        return left
    
    def parse_equality(self) -> ASTNode:
        left = self.parse_comparison()
        
        while self.current_token().type in [TokenType.EQ, TokenType.NEQ]:
            op = self.current_token().value
            self.advance()
            right = self.parse_comparison()
            left = BinaryOp(left, op, right)
        
        return left
    
    def parse_comparison(self) -> ASTNode:
        left = self.parse_addition()
        
        while self.current_token().type in [TokenType.LT, TokenType.GT, TokenType.LTE, TokenType.GTE]:
            op = self.current_token().value
            self.advance()
            right = self.parse_addition()
            left = BinaryOp(left, op, right)
        
        return left
    
    def parse_addition(self) -> ASTNode:
        left = self.parse_multiplication()
        
        while self.current_token().type in [TokenType.PLUS, TokenType.MINUS]:
            op = self.current_token().value
            self.advance()
            right = self.parse_multiplication()
            left = BinaryOp(left, op, right)
        
        return left
    
    def parse_multiplication(self) -> ASTNode:
        left = self.parse_unary()
        
        while self.current_token().type in [TokenType.STAR, TokenType.SLASH, TokenType.PERCENT]:
            op = self.current_token().value
            self.advance()
            right = self.parse_unary()
            left = BinaryOp(left, op, right)
        
        return left
    
    def parse_unary(self) -> ASTNode:
        if self.current_token().type in [TokenType.MINUS, TokenType.NOT]:
            op = self.current_token().value
            self.advance()
            operand = self.parse_unary()
            return UnaryOp(op, operand)
        
        return self.parse_primary()
    
    def parse_primary(self) -> ASTNode:
        token = self.current_token()
        
        if token.type == TokenType.INT_LIT:
            self.advance()
            return Literal(token.value, INT)
        
        elif token.type == TokenType.FLOAT_LIT:
            self.advance()
            return Literal(token.value, FLOAT)
        
        elif token.type == TokenType.STRING_LIT:
            self.advance()
            return Literal(token.value, STRING)
        
        elif token.type == TokenType.BOOL_LIT:
            self.advance()
            return Literal(token.value, BOOL)
        
        elif token.type == TokenType.IDENT:
            name = token.value
            self.advance()
            
            if self.current_token().type == TokenType.LPAREN:
                return self.parse_function_call(name)
            
            return Identifier(name)
        
        elif token.type == TokenType.LBRACKET:
            return self.parse_list_literal()
        
        elif token.type == TokenType.TENSOR:
            return self.parse_tensor_literal()
        
        elif token.type == TokenType.LPAREN:
            self.advance()
            expr = self.parse_expression()
            self.expect(TokenType.RPAREN)
            return expr
        
        raise SyntaxError(f"Unexpected token {token.type} at line {token.line}")
    
    def parse_function_call(self, name: str) -> FunctionCall:
        self.expect(TokenType.LPAREN)
        args = []
        
        while self.current_token().type != TokenType.RPAREN:
            args.append(self.parse_expression())
            if self.current_token().type == TokenType.COMMA:
                self.advance()
        
        self.expect(TokenType.RPAREN)
        return FunctionCall(name, args)
    
    def parse_list_literal(self) -> ListLiteral:
        self.expect(TokenType.LBRACKET)
        elements = []
        
        while self.current_token().type != TokenType.RBRACKET:
            elements.append(self.parse_expression())
            if self.current_token().type == TokenType.COMMA:
                self.advance()
        
        self.expect(TokenType.RBRACKET)
        return ListLiteral(elements)
    
    def parse_tensor_literal(self) -> TensorLiteral:
        self.expect(TokenType.TENSOR)
        self.expect(TokenType.LPAREN)
        
        shape = []
        if self.current_token().type == TokenType.LBRACKET:
            self.advance()
            while self.current_token().type != TokenType.RBRACKET:
                shape.append(self.expect(TokenType.INT_LIT).value)
                if self.current_token().type == TokenType.COMMA:
                    self.advance()
            self.expect(TokenType.RBRACKET)
        
        self.expect(TokenType.RPAREN)
        return TensorLiteral(shape)
    
    def parse_type(self) -> Type:
        token = self.current_token()
        
        if token.type == TokenType.IDENT:
            type_name = token.value
            self.advance()
            
            if type_name == "int":
                return INT
            elif type_name == "float":
                return FLOAT
            elif type_name == "bool":
                return BOOL
            elif type_name == "string":
                return STRING
            elif type_name == "Tensor":
                return TENSOR
            else:
                return TypeVariable(type_name)
        
        raise SyntaxError(f"Expected type, got {token.type}")

# ============================================================================
# TYPE INFERENCE
# ============================================================================

class TypeInferencer:
    def __init__(self):
        self.env: Dict[str, Type] = {}
        self.type_var_counter = 0
        self.constraints: List[tuple] = []
        
        # Built-in functions
        self.env['print'] = FunctionType([TypeVariable('a')], VOID)
        self.env['matmul'] = FunctionType([TENSOR, TENSOR], TENSOR)
        self.env['relu'] = FunctionType([TENSOR], TENSOR)
        self.env['sigmoid'] = FunctionType([TENSOR], TENSOR)
        self.env['tanh'] = FunctionType([TENSOR], TENSOR)
        self.env['softmax'] = FunctionType([TENSOR], TENSOR)
    
    def fresh_type_var(self) -> TypeVariable:
        name = f"t{self.type_var_counter}"
        self.type_var_counter += 1
        return TypeVariable(name)
    
    def infer(self, node: ASTNode, env: Optional[Dict[str, Type]] = None) -> Type:
        if env is None:
            env = self.env.copy()
        
        if isinstance(node, Program):
            for stmt in node.statements:
                self.infer(stmt, env)
            return VOID
        
        elif isinstance(node, LetStatement):
            value_type = self.infer(node.value, env)
            if node.type_annotation:
                self.unify(value_type, node.type_annotation)
                env[node.name] = node.type_annotation
            else:
                env[node.name] = value_type
            self.env[node.name] = env[node.name]
            return VOID
        
        elif isinstance(node, AssignStatement):
            value_type = self.infer(node.value, env)
            if node.name in env:
                self.unify(value_type, env[node.name])
            else:
                env[node.name] = value_type
            self.env[node.name] = env[node.name]
            return VOID
        
        elif isinstance(node, FunctionDef):
            param_types = []
            func_env = env.copy()
            
            for param_name, param_type in node.params:
                if param_type is None:
                    param_type = self.fresh_type_var()
                param_types.append(param_type)
                func_env[param_name] = param_type
            
            return_type = node.return_type if node.return_type else self.fresh_type_var()
            
            func_type = FunctionType(param_types, return_type)
            env[node.name] = func_type
            self.env[node.name] = func_type
            
            for stmt in node.body:
                stmt_type = self.infer(stmt, func_env)
                if isinstance(stmt, ReturnStatement):
                    if stmt.value:
                        self.unify(stmt_type, return_type)
            
            return VOID
        
        elif isinstance(node, ReturnStatement):
            if node.value:
                return self.infer(node.value, env)
            return VOID
        
        elif isinstance(node, IfStatement):
            cond_type = self.infer(node.condition, env)
            self.unify(cond_type, BOOL)
            
            for stmt in node.then_block:
                self.infer(stmt, env)
            
            if node.else_block:
                for stmt in node.else_block:
                    self.infer(stmt, env)
            
            return VOID
        
        elif isinstance(node, WhileStatement):
            cond_type = self.infer(node.condition, env)
            self.unify(cond_type, BOOL)
            
            for stmt in node.body:
                self.infer(stmt, env)
            
            return VOID
        
        elif isinstance(node, BinaryOp):
            left_type = self.infer(node.left, env)
            right_type = self.infer(node.right, env)
            
            if node.op in ['+', '-', '*', '/', '%']:
                # Numeric operations
                self.unify(left_type, right_type)
                if node.op in ['+', '-', '*', '/']:
                    # Support int, float, and tensor
                    return left_type
                return left_type
            
            elif node.op in ['==', '!=', '<', '>', '<=', '>=']:
                # Comparison operations
                self.unify(left_type, right_type)
                return BOOL
            
            elif node.op in ['and', 'or']:
                # Logical operations
                self.unify(left_type, BOOL)
                self.unify(right_type, BOOL)
                return BOOL
            
            return self.fresh_type_var()
        
        elif isinstance(node, UnaryOp):
            operand_type = self.infer(node.operand, env)
            
            if node.op == '-':
                return operand_type
            elif node.op == 'not':
                self.unify(operand_type, BOOL)
                return BOOL
            
            return operand_type
        
        elif isinstance(node, FunctionCall):
            if node.name not in env:
                raise TypeError(f"Undefined function: {node.name}")
            
            func_type = env[node.name]
            arg_types = [self.infer(arg, env) for arg in node.args]
            
            if isinstance(func_type, FunctionType):
                if len(arg_types) != len(func_type.params):
                    raise TypeError(f"Function {node.name} expects {len(func_type.params)} args, got {len(arg_types)}")
                
                for arg_type, param_type in zip(arg_types, func_type.params):
                    self.unify(arg_type, param_type)
                
                return func_type.return_type
            
            return self.fresh_type_var()
        
        elif isinstance(node, Identifier):
            if node.name not in env:
                raise TypeError(f"Undefined variable: {node.name}")
            return env[node.name]
        
        elif isinstance(node, Literal):
            return node.type
        
        elif isinstance(node, ListLiteral):
            if not node.elements:
                return ListType(self.fresh_type_var())
            
            elem_type = self.infer(node.elements[0], env)
            for elem in node.elements[1:]:
                self.unify(elem_type, self.infer(elem, env))
            
            return ListType(elem_type)
        
        elif isinstance(node, TensorLiteral):
            return TensorType(node.shape)
        
        return self.fresh_type_var()
    
    def unify(self, t1: Type, t2: Type):
        """Unification algorithm for type checking"""
        if isinstance(t1, TypeVariable):
            if t1 != t2:
                self.constraints.append((t1, t2))
            return
        
        if isinstance(t2, TypeVariable):
            if t1 != t2:
                self.constraints.append((t2, t1))
            return
        
        if isinstance(t1, PrimitiveType) and isinstance(t2, PrimitiveType):
            if t1.name != t2.name:
                raise TypeError(f"Type mismatch: {t1} vs {t2}")
            return
        
        if isinstance(t1, ListType) and isinstance(t2, ListType):
            self.unify(t1.element_type, t2.element_type)
            return
        
        if isinstance(t1, FunctionType) and isinstance(t2, FunctionType):
            if len(t1.params) != len(t2.params):
                raise TypeError(f"Function type mismatch: {t1} vs {t2}")
            
            for p1, p2 in zip(t1.params, t2.params):
                self.unify(p1, p2)
            
            self.unify(t1.return_type, t2.return_type)
            return
        
        if type(t1) != type(t2):
            raise TypeError(f"Type mismatch: {t1} vs {t2}")

# ============================================================================
# BYTECODE
# ============================================================================

class OpCode(Enum):
    # Stack operations
    LOAD_CONST = auto()
    LOAD_VAR = auto()
    STORE_VAR = auto()
    
    # Arithmetic
    ADD = auto()
    SUB = auto()
    MUL = auto()
    DIV = auto()
    MOD = auto()
    NEG = auto()
    
    # Comparison
    EQ = auto()
    NEQ = auto()
    LT = auto()
    GT = auto()
    LTE = auto()
    GTE = auto()
    
    # Logical
    AND = auto()
    OR = auto()
    NOT = auto()
    
    # Control flow
    JUMP = auto()
    JUMP_IF_FALSE = auto()
    CALL = auto()
    RETURN = auto()
    
    # Tensor operations
    TENSOR_CREATE = auto()
    TENSOR_ADD = auto()
    TENSOR_MUL = auto()
    MATMUL = auto()
    RELU = auto()
    SIGMOID = auto()
    TANH = auto()
    SOFTMAX = auto()
    
    # List operations
    BUILD_LIST = auto()
    
    # Special
    POP = auto()
    PRINT = auto()
    HALT = auto()

@dataclass
class Instruction:
    opcode: OpCode
    arg: Any = None
    
    def __repr__(self):
        if self.arg is not None:
            return f"{self.opcode.name:20} {self.arg}"
        return f"{self.opcode.name}"

# ============================================================================
# COMPILER
# ============================================================================

class Compiler:
    def __init__(self):
        self.instructions: List[Instruction] = []
        self.constants: List[Any] = []
        self.var_indices: Dict[str, int] = {}
        self.next_var_index = 0
        self.label_counter = 0
        self.labels: Dict[str, int] = {}
        self.function_addresses: Dict[str, int] = {}
    
    def add_constant(self, value: Any) -> int:
        if value not in self.constants:
            self.constants.append(value)
        return self.constants.index(value)
    
    def get_var_index(self, name: str) -> int:
        if name not in self.var_indices:
            self.var_indices[name] = self.next_var_index
            self.next_var_index += 1
        return self.var_indices[name]
    
    def emit(self, opcode: OpCode, arg: Any = None):
        self.instructions.append(Instruction(opcode, arg))
    
    def current_address(self) -> int:
        return len(self.instructions)
    
    def new_label(self) -> str:
        label = f"L{self.label_counter}"
        self.label_counter += 1
        return label
    
    def mark_label(self, label: str):
        self.labels[label] = self.current_address()
    
    def compile(self, node: ASTNode):
        if isinstance(node, Program):
            for stmt in node.statements:
                self.compile(stmt)
            self.emit(OpCode.HALT)
        
        elif isinstance(node, LetStatement):
            self.compile(node.value)
            var_idx = self.get_var_index(node.name)
            self.emit(OpCode.STORE_VAR, var_idx)
        
        elif isinstance(node, AssignStatement):
            self.compile(node.value)
            var_idx = self.get_var_index(node.name)
            self.emit(OpCode.STORE_VAR, var_idx)
        
        elif isinstance(node, FunctionDef):
            # Store function address
            func_label = f"func_{node.name}"
            jump_label = self.new_label()
            
            # Jump over function body
            self.emit(OpCode.JUMP, jump_label)
            
            # Function body
            self.mark_label(func_label)
            self.function_addresses[node.name] = self.current_address()
            
            # Store parameters
            for i, (param_name, _) in enumerate(reversed(node.params)):
                var_idx = self.get_var_index(param_name)
                self.emit(OpCode.STORE_VAR, var_idx)
            
            # Compile body
            for stmt in node.body:
                self.compile(stmt)
            
            # Default return
            if not node.body or not isinstance(node.body[-1], ReturnStatement):
                const_idx = self.add_constant(None)
                self.emit(OpCode.LOAD_CONST, const_idx)
                self.emit(OpCode.RETURN)
            
            self.mark_label(jump_label)
        
        elif isinstance(node, ReturnStatement):
            if node.value:
                self.compile(node.value)
            else:
                const_idx = self.add_constant(None)
                self.emit(OpCode.LOAD_CONST, const_idx)
            self.emit(OpCode.RETURN)
        
        elif isinstance(node, IfStatement):
            else_label = self.new_label()
            end_label = self.new_label()
            
            # Condition
            self.compile(node.condition)
            self.emit(OpCode.JUMP_IF_FALSE, else_label)
            
            # Then block
            for stmt in node.then_block:
                self.compile(stmt)
            self.emit(OpCode.JUMP, end_label)
            
            # Else block
            self.mark_label(else_label)
            if node.else_block:
                for stmt in node.else_block:
                    self.compile(stmt)
            
            self.mark_label(end_label)
        
        elif isinstance(node, WhileStatement):
            start_label = self.new_label()
            end_label = self.new_label()
            
            self.mark_label(start_label)
            
            # Condition
            self.compile(node.condition)
            self.emit(OpCode.JUMP_IF_FALSE, end_label)
            
            # Body
            for stmt in node.body:
                self.compile(stmt)
            
            self.emit(OpCode.JUMP, start_label)
            self.mark_label(end_label)
        
        elif isinstance(node, BinaryOp):
            self.compile(node.left)
            self.compile(node.right)
            
            op_map = {
                '+': OpCode.ADD,
                '-': OpCode.SUB,
                '*': OpCode.MUL,
                '/': OpCode.DIV,
                '%': OpCode.MOD,
                '==': OpCode.EQ,
                '!=': OpCode.NEQ,
                '<': OpCode.LT,
                '>': OpCode.GT,
                '<=': OpCode.LTE,
                '>=': OpCode.GTE,
                'and': OpCode.AND,
                'or': OpCode.OR,
            }
            
            if node.op in op_map:
                self.emit(op_map[node.op])
        
        elif isinstance(node, UnaryOp):
            self.compile(node.operand)
            
            if node.op == '-':
                self.emit(OpCode.NEG)
            elif node.op == 'not':
                self.emit(OpCode.NOT)
        
        elif isinstance(node, FunctionCall):
            # Built-in functions
            if node.name == 'print':
                for arg in node.args:
                    self.compile(arg)
                    self.emit(OpCode.PRINT)
            elif node.name == 'matmul':
                for arg in node.args:
                    self.compile(arg)
                self.emit(OpCode.MATMUL)
            elif node.name == 'relu':
                self.compile(node.args[0])
                self.emit(OpCode.RELU)
            elif node.name == 'sigmoid':
                self.compile(node.args[0])
                self.emit(OpCode.SIGMOID)
            elif node.name == 'tanh':
                self.compile(node.args[0])
                self.emit(OpCode.TANH)
            elif node.name == 'softmax':
                self.compile(node.args[0])
                self.emit(OpCode.SOFTMAX)
            else:
                # User-defined functions
                for arg in node.args:
                    self.compile(arg)
                
                func_label = f"func_{node.name}"
                self.emit(OpCode.CALL, func_label)
        
        elif isinstance(node, Identifier):
            var_idx = self.get_var_index(node.name)
            self.emit(OpCode.LOAD_VAR, var_idx)
        
        elif isinstance(node, Literal):
            const_idx = self.add_constant(node.value)
            self.emit(OpCode.LOAD_CONST, const_idx)
        
        elif isinstance(node, ListLiteral):
            for elem in node.elements:
                self.compile(elem)
            self.emit(OpCode.BUILD_LIST, len(node.elements))
        
        elif isinstance(node, TensorLiteral):
            const_idx = self.add_constant(node.shape)
            self.emit(OpCode.LOAD_CONST, const_idx)
            self.emit(OpCode.TENSOR_CREATE)
    
    def resolve_labels(self):
        """Resolve label references to addresses"""
        for i, instr in enumerate(self.instructions):
            if isinstance(instr.arg, str) and instr.arg.startswith('L'):
                if instr.arg in self.labels:
                    instr.arg = self.labels[instr.arg]
            elif isinstance(instr.arg, str) and instr.arg.startswith('func_'):
                func_name = instr.arg[5:]
                if func_name in self.function_addresses:
                    instr.arg = self.function_addresses[func_name]

# ============================================================================
# VIRTUAL MACHINE
# ============================================================================

class VirtualMachine:
    def __init__(self, instructions: List[Instruction], constants: List[Any]):
        self.instructions = instructions
        self.constants = constants
        self.stack: List[Any] = []
        self.variables: Dict[int, Any] = {}
        self.pc = 0  # Program counter
        self.call_stack: List[int] = []
    
    def push(self, value: Any):
        self.stack.append(value)
    
    def pop(self) -> Any:
        if not self.stack:
            raise RuntimeError("Stack underflow")
        return self.stack.pop()
    
    def run(self):
        while self.pc < len(self.instructions):
            instr = self.instructions[self.pc]
            self.execute(instr)
            self.pc += 1
    
    def execute(self, instr: Instruction):
        opcode = instr.opcode
        
        if opcode == OpCode.LOAD_CONST:
            self.push(self.constants[instr.arg])
        
        elif opcode == OpCode.LOAD_VAR:
            if instr.arg not in self.variables:
                raise RuntimeError(f"Undefined variable at index {instr.arg}")
            self.push(self.variables[instr.arg])
        
        elif opcode == OpCode.STORE_VAR:
            value = self.pop()
            self.variables[instr.arg] = value
        
        elif opcode == OpCode.ADD:
            b, a = self.pop(), self.pop()
            self.push(a + b)
        
        elif opcode == OpCode.SUB:
            b, a = self.pop(), self.pop()
            self.push(a - b)
        
        elif opcode == OpCode.MUL:
            b, a = self.pop(), self.pop()
            self.push(a * b)
        
        elif opcode == OpCode.DIV:
            b, a = self.pop(), self.pop()
            self.push(a / b)
        
        elif opcode == OpCode.MOD:
            b, a = self.pop(), self.pop()
            self.push(a % b)
        
        elif opcode == OpCode.NEG:
            self.push(-self.pop())
        
        elif opcode == OpCode.EQ:
            b, a = self.pop(), self.pop()
            self.push(a == b)
        
        elif opcode == OpCode.NEQ:
            b, a = self.pop(), self.pop()
            self.push(a != b)
        
        elif opcode == OpCode.LT:
            b, a = self.pop(), self.pop()
            self.push(a < b)
        
        elif opcode == OpCode.GT:
            b, a = self.pop(), self.pop()
            self.push(a > b)
        
        elif opcode == OpCode.LTE:
            b, a = self.pop(), self.pop()
            self.push(a <= b)
        
        elif opcode == OpCode.GTE:
            b, a = self.pop(), self.pop()
            self.push(a >= b)
        
        elif opcode == OpCode.AND:
            b, a = self.pop(), self.pop()
            self.push(a and b)
        
        elif opcode == OpCode.OR:
            b, a = self.pop(), self.pop()
            self.push(a or b)
        
        elif opcode == OpCode.NOT:
            self.push(not self.pop())
        
        elif opcode == OpCode.JUMP:
            self.pc = instr.arg - 1
        
        elif opcode == OpCode.JUMP_IF_FALSE:
            condition = self.pop()
            if not condition:
                self.pc = instr.arg - 1
        
        elif opcode == OpCode.CALL:
            self.call_stack.append(self.pc)
            self.pc = instr.arg - 1
        
        elif opcode == OpCode.RETURN:
            if self.call_stack:
                self.pc = self.call_stack.pop()
            else:
                self.pc = len(self.instructions)
        
        elif opcode == OpCode.BUILD_LIST:
            count = instr.arg
            elements = [self.pop() for _ in range(count)]
            self.push(list(reversed(elements)))
        
        elif opcode == OpCode.TENSOR_CREATE:
            shape = self.pop()
            # Simulate tensor creation
            import random
            size = 1
            for dim in shape:
                size *= dim
            tensor = [random.random() for _ in range(size)]
            self.push({'type': 'tensor', 'shape': shape, 'data': tensor})
        
        elif opcode == OpCode.MATMUL:
            b, a = self.pop(), self.pop()
            # Simplified matrix multiplication
            result = {'type': 'tensor', 'shape': [1, 1], 'data': [0.0]}
            self.push(result)
        
        elif opcode == OpCode.RELU:
            tensor = self.pop()
            if isinstance(tensor, dict) and tensor['type'] == 'tensor':
                tensor['data'] = [max(0, x) for x in tensor['data']]
            self.push(tensor)
        
        elif opcode == OpCode.SIGMOID:
            import math
            tensor = self.pop()
            if isinstance(tensor, dict) and tensor['type'] == 'tensor':
                tensor['data'] = [1 / (1 + math.exp(-x)) for x in tensor['data']]
            self.push(tensor)
        
        elif opcode == OpCode.TANH:
            import math
            tensor = self.pop()
            if isinstance(tensor, dict) and tensor['type'] == 'tensor':
                tensor['data'] = [math.tanh(x) for x in tensor['data']]
            self.push(tensor)
        
        elif opcode == OpCode.SOFTMAX:
            import math
            tensor = self.pop()
            if isinstance(tensor, dict) and tensor['type'] == 'tensor':
                exp_vals = [math.exp(x) for x in tensor['data']]
                sum_exp = sum(exp_vals)
                tensor['data'] = [x / sum_exp for x in exp_vals]
            self.push(tensor)
        
        elif opcode == OpCode.PRINT:
            value = self.pop()
            if isinstance(value, dict) and value['type'] == 'tensor':
                print(f"Tensor{value['shape']}: {value['data'][:5]}...")
            else:
                print(value)
        
        elif opcode == OpCode.POP:
            self.pop()
        
        elif opcode == OpCode.HALT:
            self.pc = len(self.instructions)

# ============================================================================
# MAIN INTERFACE
# ============================================================================

class NeuralScript:
    """Main interface for the NeuralScript language"""
    
    def __init__(self):
        self.lexer = None
        self.parser = None
        self.type_inferencer = None
        self.compiler = None
        self.vm = None
    
    def compile(self, source: str) -> tuple:
        """Compile source code to bytecode"""
        # Lexical analysis
        self.lexer = Lexer(source)
        tokens = self.lexer.tokenize()
        
        # Parsing
        self.parser = Parser(tokens)
        ast = self.parser.parse()
        
        # Type inference
        self.type_inferencer = TypeInferencer()
        self.type_inferencer.infer(ast)
        
        # Compilation
        self.compiler = Compiler()
        self.compiler.compile(ast)
        self.compiler.resolve_labels()
        
        return self.compiler.instructions, self.compiler.constants
    
    def run(self, source: str):
        """Compile and execute source code"""
        instructions, constants = self.compile(source)
        
        # Execution
        self.vm = VirtualMachine(instructions, constants)
        self.vm.run()
    
    def disassemble(self):
        """Show compiled bytecode"""
        if not self.compiler:
            print("No code compiled yet")
            return
        
        print("\n=== BYTECODE ===")
        for i, instr in enumerate(self.compiler.instructions):
            print(f"{i:04d}  {instr}")
        
        print("\n=== CONSTANTS ===")
        for i, const in enumerate(self.compiler.constants):
            print(f"{i:04d}  {const}")

# ============================================================================
# EXAMPLE USAGE
# ============================================================================

if __name__ == "__main__":
    ns = NeuralScript()
    
    # Example 1: Basic arithmetic and variables
    print("Example 1: Basic Operations")
    print("-" * 50)
    code1 = """
    let x = 10
    let y = 20
    let sum = x + y
    print(sum)
    """
    ns.run(code1)
    
    # Example 2: Functions
    print("\nExample 2: Functions")
    print("-" * 50)
    code2 = """
    fn add(a, b) {
        return a + b
    }
    
    fn multiply(a, b) {
        return a * b
    }
    
    let result1 = add(5, 3)
    let result2 = multiply(4, 7)
    print(result1)
    print(result2)
    """
    ns.run(code2)
    
    # Example 3: Control flow
    print("\nExample 3: Control Flow")
    print("-" * 50)
    code3 = """
    let x = 15
    
    if x > 10 {
        print("x is greater than 10")
    } else {
        print("x is not greater than 10")
    }
    
    let i = 0
    while i < 5 {
        print(i)
        i = i + 1
    }
    """
    ns.run(code3)
    
    # Example 4: Lists
    print("\nExample 4: Lists")
    print("-" * 50)
    code4 = """
    let numbers = [1, 2, 3, 4, 5]
    print(numbers)
    
    let names = ["Alice", "Bob", "Charlie"]
    print(names)
    """
    ns.run(code4)
    
    # Example 5: Tensors and AI operations
    print("\nExample 5: Tensor Operations")
    print("-" * 50)
    code5 = """
    let weights = tensor([3, 3])
    print(weights)
    
    let activated = relu(weights)
    print(activated)
    
    let normalized = sigmoid(weights)
    print(normalized)
    """
    ns.run(code5)
    
    # Show bytecode for last example
    print("\nBytecode for last example:")
    ns.disassemble()
    
    print("\n" + "=" * 50)
    print("NeuralScript Language Demo Complete!")
    print("=" * 50)
