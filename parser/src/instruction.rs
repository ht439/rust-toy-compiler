use super::ast::{Expr, Literal, OpType, OrderType, Declaration, Stmt};
use super::expression::expr;
use super::literal::parse_identifier;
use super::literal::parse_string;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0, multispace1, space0, space1},
    combinator::{eof, map, opt, peek, recognize},
    error::{FromExternalError, ParseError},
    multi::{many0, separated_list0},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};
use std::borrow::Cow;
use std::num::ParseFloatError;

pub fn program<'i, E>(input: &'i str) -> IResult<&'i str, Vec<Declaration>, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    delimited(multispace0, many0(declaration),  eof)(input)
}

fn stmt_end<'i, E: ParseError<&'i str>>(input: &'i str) -> IResult<&'i str, &'i str, E> {
    alt((multispace1, eof, recognize(peek(char('}')))))(input)
}

// Declaration parsers
pub(crate) fn declaration<'i, E>(input: &'i str) -> IResult<&'i str, Declaration, E>
where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError> 
{
    alt((var_decl, fun_decl, statement))(input)
}

pub(crate) fn fun_decl<'i, E>(input: &'i str) -> IResult<&'i str, Declaration, E>
    where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError> 
{
    preceded(tag("fn"), function)(input)
}

pub(crate) fn var_decl<'i, E>(input: &'i str) -> IResult<&'i str, Declaration, E>
    where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    map(
        preceded(
            tag("let"), 
            terminated(
                preceded(
                    space1,
                    pair(
                        parse_identifier,
                        opt(pair(
                        delimited(space0, char('='), space0),
                        expr))
                    )
                ),
                stmt_end
            ),
        ),
        |(id, exp)| {
            let parsed_id = Box::new(Expr::Id(id.into()));
            match &exp {
                Some((_, e)) => Declaration::VarDecl(parsed_id, Some(e.to_owned())),
                None                => Declaration::VarDecl(parsed_id, None),
            }
        }
    )
    (input)
}

//statement parsers
pub(crate) fn statement<'i, E>(input: &'i str) -> IResult<&'i str, Declaration, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    map(
        alt((while_stmt, print_stmt, block, call_stmt, expr_stmt)),
        |stmt| Declaration::Statement(Box::new(stmt))
    )(input)
}

pub(crate) fn expr_stmt<'i, E>(input: &'i str) -> IResult<&'i str, Stmt, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    map(terminated(expr, stmt_end), |exp| Stmt::ExprStmt(exp))(input)
}

pub(crate) fn call_stmt<'i, E>(input: &'i str) -> IResult<&'i str, Stmt, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    map(
        terminated(
            pair(
                parse_identifier,
                delimited(
                    preceded(space0, char('(')),
                    separated_list0(char(','), expr),
                    preceded(space0, char(')')),
                ),
            ),
            stmt_end,
        ),
        |(name, args)| Stmt::Call(name.into(), args),
    )(input)
}

pub(crate) fn print_stmt<'i, E>(input: &'i str) -> IResult<&'i str, Stmt, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    map(
        terminated(
            preceded(
                tag("print"),
                delimited(
                    char('('),
                    delimited(space0, alt((expr, parse_string)), space0),
                    char(')'),
                ),
            ),
            stmt_end
        ),
        |exp| Stmt::Print(exp),
    )(input)
}

pub(crate) fn while_stmt<'i, E>(input: &'i str) -> IResult<&'i str, Stmt, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    let (input,  exp) =
        terminated(
            preceded(
                tag("while"),
                delimited(
                    preceded(space0, char('(')),
                    delimited(space0, expr, space0),
                    char(')'),
                )
            ),
            multispace0)
    (input)?;
    let (input, list) = statement(input)?;

    let body = match list {
        Declaration::Statement(blk) => blk,
        _ => {
            return Err(nom::Err::Error(E::from_error_kind(
                input,
                nom::error::ErrorKind::Verify,
            )))
        }
    };
    Ok((input, Stmt::While(exp, body)))
}

pub(crate) fn block<'i, E>(input: &'i str) -> IResult<&'i str, Stmt, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    map(
        tuple((
            terminated(tag("{"), multispace0),
            many0(declaration),
            terminated(tag("}"), stmt_end),
        )),
        |(_, decl_list, _)| Stmt::Block(decl_list)
    )(input)
}

pub(crate) fn function<'i, E>(input: &'i str) -> IResult<&'i str, Declaration, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    map(
        tuple((
            space1,
            parse_identifier,
            preceded(space0, char('(')),
            separated_list0(char(','), delimited(space0, parse_identifier, space0)),
            preceded(space0, char(')')),
            preceded(multispace0, block),
        )),
        move |(_, name, _, params, _, blck)| Declaration::FuncDecl(name.into(), params, Box::new(blck)),
    )(input)
}

mod test {
    use super::*;
    use nom::error::Error;

    #[test]
    fn test_print_stmt() {
        let test1 = print_stmt::<Error<&str>>("print(x)\n").unwrap();
        let test2 = print_stmt::<Error<&str>>("print(\"test\")\n").unwrap();
        let test3 = print_stmt::<Error<&str>>("print(x + y)\n").unwrap();
        assert_eq!(test1, ("", Stmt::Print(Expr::Id(Cow::Borrowed("x")))));
        assert_eq!(
            test2,
            ("", Stmt::Print(Expr::ExprLiteral(Literal::StringLiteral(Cow::Borrowed("test")))))
        );
        assert_eq!(
            test3,
            (
                "",
                Stmt::Print(Expr::OpExpr(
                    OpType::Add,
                    Box::new(Expr::Id(Cow::Borrowed("x"))),
                    Box::new(Expr::Id(Cow::Borrowed("y"))),
                ))
            )
        );
    }
}