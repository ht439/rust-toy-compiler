use super::literal::{parse_identifier, parse_number, parse_bool, ident_char};
use super::ast::{Expr, OpType, OrderType, UnaryType, Literal};
use std::borrow::Cow;
use nom::{
    branch::alt,
    sequence::{delimited, separated_pair, pair, terminated},
    bytes::complete::tag,
    character::complete::{char, space0},
    combinator::{map, not, peek},
    error::{FromExternalError, ParseError},
    multi::fold_many0,
    IResult,
};
use super::literal::parse_string;
use std::num::ParseFloatError;

pub(crate) fn logic_or<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    let (cons, prod) = logic_and(input)?;
    fold_many0(
        pair(terminated(tag("or"), peek(not(ident_char))), logic_and),
        move || prod.clone(),
        |acc, (_op, e)| Expr::OrderExpr(OrderType::Or, Box::new(acc), Box::new(e)),
    )(cons)
}

pub(crate) fn logic_and<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    let (cons, prod) = equality(input)?;
    fold_many0(
        pair(terminated(tag("and"), peek(not(ident_char))), equality),
        move || prod.clone(),
        |acc, (_op, e)| Expr::OrderExpr(OrderType::And, Box::new(acc), Box::new(e)),
    )(cons)
}

pub(crate) fn assignment<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    alt((
        map(
            separated_pair(
                parse_identifier,
                delimited(space0, char('='), space0),
                assignment,
            ),
            move |(id, exp)|
                return Expr::Assign(Box::new(Expr::Id(Cow::Borrowed(id))), Box::new(exp))
        ),
        logic_or
    ))(input)
}

pub(crate) fn equality<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    let (cons, prod) = comparison(input)?;
    fold_many0(
        pair(equality_operator, comparison),
        move || prod.clone(),
        |acc, (op, e)| match op {
           "==" => Expr::OrderExpr(OrderType::Equal, Box::new(acc), Box::new(e)),
           "!=" => Expr::OrderExpr(OrderType::Unequal, Box::new(acc), Box::new(e)),
            _  => panic!("error, unable to parse operator"),  
        }
    )(cons)
}

pub(crate) fn comparison<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>,
{
    let (cons, prod) = term(input)?;
    fold_many0(
        pair(alt((tag("<="), tag(">="), tag("<"), tag(">"))), term),
        move || prod.clone(),
        |acc, (op, e)| {
            return Expr::OrderExpr(OrderType::from(op), Box::new(acc), Box::new(e))
        },
    )(cons)
}

fn parens<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    delimited(tag("("), 
        delimited(space0, expr, space0),
    tag(")"))(input)
}

pub(crate) fn primary<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E> 
    where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    alt((
        map(delimited(space0, parse_number, space0),
            |num: Literal| Expr::ExprLiteral(num)),
        // `parse_bool` must precede `parse_identifier`, otherwise `true`/`false`
        // are swallowed as identifiers and never parse as boolean literals.
        map(delimited(space0, parse_bool, space0),
            |boolean: Literal| Expr::ExprLiteral(boolean)),
        map(delimited(space0, parse_identifier, space0),
            |id: &str| Expr::Id(Cow::Borrowed(id))),
        delimited(space0, parse_string, space0),
        delimited(space0, parens, space0),
    ))(input)
}

pub(crate) fn factor_operator<'i, E>(input: &'i str) -> IResult<&'i str, char, E>
    where E: ParseError<&'i str>
{
    alt((char('/'), char('*'), char('%')))(input)
}

pub(crate) fn term_operator<'i, E>(input: &'i str) -> IResult<&'i str, char, E>
    where E: ParseError<&'i str>
{
    alt((char('+'), char('-')))(input)
}

pub(crate) fn unary_operator<'i, E>(input: &'i str) -> IResult<&'i str, char, E>
    where E: ParseError<&'i str>
{
    alt((char('!'), char('-')))(input)
}

pub(crate) fn equality_operator<'i, E>(input: &'i str) -> IResult<&'i str, &'i str, E>
    where E: ParseError<&'i str>
{
    alt((tag("=="), tag("!=")))(input)
}

pub(crate) fn term<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
    where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    let (cons, prod) = factor(input)?;
    fold_many0(
        pair(term_operator, factor),
        move || prod.clone(),
        |acc, (op, e)| match op {
           '+' => Expr::OpExpr(OpType::Add, Box::new(acc), Box::new(e)),
           '-' => Expr::OpExpr(OpType::Sub, Box::new(acc), Box::new(e)),
            _  => panic!("error, unable to parse operator"),  
        }
    )(cons)
}

pub(crate) fn factor<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
    where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    let (cons, prod) = unary(input)?;
    fold_many0(
        pair(factor_operator, unary),
        move || prod.clone(),
        |acc, (op, e)| match op {
           '*' => Expr::OpExpr(OpType::Mul, Box::new(acc), Box::new(e)),
           '/' => Expr::OpExpr(OpType::Div, Box::new(acc), Box::new(e)),
           '%' => Expr::OpExpr(OpType::Mod, Box::new(acc), Box::new(e)),
            _  => panic!("error, unable to parse operator"),  
        }
    )(cons)
}

pub(crate) fn unary<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
    where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    alt((
        map(pair(unary_operator, primary),
         |exp| {
            match exp {
                ('!', e) => Expr::UnaryExpr(UnaryType::Not, Box::new(e)),
                ('-', e) => Expr::UnaryExpr(UnaryType::Negative, Box::new(e)),
                _ => panic!("error, unable to parse operator"),
            }
         }),
        primary
    ))(input)
}

pub(crate) fn expr<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
    where E: ParseError<&'i str> + FromExternalError<&'i str, ParseFloatError>
{
    assignment(input)
}

#[cfg(test)]
mod test {
    use super::*;
    use nom::error::Error;

    #[test]
    fn test_expr() {
        let test1 = expr::<Error<&str>>(" (  2 )").unwrap();
        let test2 = expr::<Error<&str>>(" ( 2 + 6 )").unwrap();
        let test3 = expr::<Error<&str>>("( x >=6) ").unwrap();
        assert_eq!(test1, ("", Expr::ExprLiteral(Literal::NumLiteral(2.0))));
        assert_eq!(test2, ("", Expr::OpExpr(
            OpType::Add, 
            Box::new(Expr::ExprLiteral(Literal::NumLiteral(2.0))), 
            Box::new(Expr::ExprLiteral(Literal::NumLiteral(6.0)))
        )));
        assert_eq!(test3, ("", Expr::OrderExpr(
            OrderType::GreaterOrEqual,
            Box::new(Expr::Id(Cow::Borrowed("x"))),
            Box::new(Expr::ExprLiteral(Literal::NumLiteral(6.0))),
        )));
    }
}
