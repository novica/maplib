use oxrdf::{BlankNode, NamedNode, Variable};
use savvy::savvy;

/// Validate an IRI's syntax (mirrors oxrdf::NamedNode::new, the same check
/// py_maplib's PyIRI::new uses, lib/representation/src/python.rs:279-282).
/// Raises an R error on invalid syntax; returns nothing on success.
///
/// @param iri IRI string to validate.
/// @export
#[savvy]
fn validate_iri(iri: &str) -> savvy::Result<()> {
    NamedNode::new(iri).map_err(|e| savvy::Error::new(&e.to_string()))?;
    Ok(())
}

/// Validate a SPARQL variable name's syntax (mirrors oxrdf::Variable::new,
/// used by py_maplib's PyVariable::new, lib/representation/src/python.rs:340-343).
/// Raises an R error on invalid syntax; returns nothing on success.
///
/// @param name Variable name, without the leading `?`/`$`.
/// @export
#[savvy]
fn validate_variable_name(name: &str) -> savvy::Result<()> {
    Variable::new(name).map_err(|e| savvy::Error::new(&e.to_string()))?;
    Ok(())
}

/// Validate a blank node identifier's syntax (mirrors oxrdf::BlankNode::new,
/// used by py_maplib's PyBlankNode::new, lib/representation/src/python.rs:531-535).
/// Raises an R error on invalid syntax; returns nothing on success.
///
/// @param id Blank node identifier, without the leading `_:`.
/// @export
#[savvy]
fn validate_blank_node_id(id: &str) -> savvy::Result<()> {
    BlankNode::new(id).map_err(|e| savvy::Error::new(&e.to_string()))?;
    Ok(())
}
