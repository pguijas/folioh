use super::*;

fn param(name: &str, ty: Option<&str>, desc: &str) -> (String, Option<String>, String) {
    (name.to_string(), ty.map(str::to_string), desc.to_string())
}

fn params_of(parsed: &ParsedDocstring) -> Vec<(String, Option<String>, String)> {
    parsed
        .params
        .iter()
        .map(|p| {
            (
                p.arg_name.clone(),
                p.type_name.clone(),
                p.description.clone(),
            )
        })
        .collect()
}

const GOOGLE_DOC: &str = "Establish a connection to the remote server.\n\nOpens a TCP connection using the specified host and port. The connection\nwill be kept alive until explicitly closed or the timeout is reached.\n\nArgs:\n    host: The hostname or IP address to connect to.\n    port: The port number. Defaults to 8080.\n    timeout: Maximum time in seconds to wait for the connection.\n\nReturns:\n    A Connection object representing the active connection.\n\nRaises:\n    ConnectionError: If the server is unreachable.\n    ValueError: If the host string is empty.\n\nExamples:\n    >>> conn = connect(\"localhost\")\n    >>> conn.is_alive()\n    True\n\nNotes:\n    This function does not support Unix domain sockets.";

const NUMPY_DOC: &str = "Establish a connection to the remote server.\n\nParameters\n----------\nhost : str\n    The hostname or IP address.\nport : int, optional\n    The port number, by default 8080.\n\nReturns\n-------\nConnection\n    The active connection object.\n\nRaises\n------\nConnectionError\n    If the server is unreachable.\n\nExamples\n--------\n>>> conn = connect(\"localhost\")\n>>> conn.is_alive()\nTrue\nSome prose.\n\nNotes\n-----\nKeeps the socket open.";

#[test]
fn resolve_style_table() {
    for (configured, expected) in [
        ("google", DocstringStyle::Google),
        ("numpy", DocstringStyle::Numpy),
        ("auto", DocstringStyle::Auto),
        ("NumPy", DocstringStyle::Google),
        ("rest", DocstringStyle::Google),
        ("unknown-style", DocstringStyle::Google),
        ("", DocstringStyle::Google),
    ] {
        assert_eq!(resolve_style(configured), expected, "{configured:?}");
    }
    assert_eq!(DocstringStyle::default(), DocstringStyle::Auto);
}

#[test]
fn google_docs_example() {
    for style in [DocstringStyle::Google, DocstringStyle::Auto] {
        let parsed = parse(GOOGLE_DOC, style);
        assert_eq!(parsed.style, ParsedStyle::Google);
        assert_eq!(
            parsed.short_description,
            "Establish a connection to the remote server."
        );
        assert_eq!(
                parsed.long_description,
                "Opens a TCP connection using the specified host and port. The connection\nwill be kept alive until explicitly closed or the timeout is reached."
            );
        assert_eq!(
            params_of(&parsed),
            vec![
                param("host", None, "The hostname or IP address to connect to."),
                param("port", None, "The port number. Defaults to 8080."),
                param(
                    "timeout",
                    None,
                    "Maximum time in seconds to wait for the connection."
                ),
            ]
        );
        assert_eq!(parsed.params[1].default.as_deref(), Some("8080"));
        assert_eq!(parsed.params[0].is_optional, None);
        let returns = parsed.returns.as_ref().unwrap();
        assert_eq!(returns.type_name, None);
        assert_eq!(
            returns.description,
            "A Connection object representing the active connection."
        );
        assert!(!returns.is_generator);
        assert_eq!(
            parsed
                .raises
                .iter()
                .map(|r| (r.type_name.clone().unwrap(), r.description.clone()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "ConnectionError".to_string(),
                    "If the server is unreachable.".to_string()
                ),
                (
                    "ValueError".to_string(),
                    "If the host string is empty.".to_string()
                ),
            ]
        );
        assert_eq!(
            parsed.examples,
            vec![">>> conn = connect(\"localhost\")\n>>> conn.is_alive()\nTrue"]
        );
        // Google `Notes:` is a section; the docs promise it.
        assert_eq!(
            parsed.notes,
            vec!["This function does not support Unix domain sockets."]
        );
    }
}

#[test]
fn google_typed_args_returns_and_item_forms() {
    let text = "G.\n\nArgs:\n    x (int): the x\n    y (str, optional): the y. Defaults to None.\n    z (Mapping[str, int]?): maybe\n    w: first\n      continued: still w\n\nReturns:\n    str: out\n\nYields:\n    int: ignored second item\n\nExample:\n    >>> g(1)\n    'out'";
    let parsed = parse(text, DocstringStyle::Google);
    assert_eq!(
        params_of(&parsed),
        vec![
            param("x", Some("int"), "the x"),
            param("y", Some("str"), "the y. Defaults to None."),
            param("z", Some("Mapping[str, int]"), "maybe"),
            param("w", None, "first\ncontinued: still w"),
        ]
    );
    assert_eq!(
        parsed
            .params
            .iter()
            .map(|p| p.is_optional)
            .collect::<Vec<_>>(),
        vec![Some(false), Some(true), Some(true), None]
    );
    assert_eq!(parsed.params[1].default.as_deref(), Some("None"));
    let returns = parsed.returns.unwrap();
    assert_eq!(returns.type_name.as_deref(), Some("str"));
    assert_eq!(returns.description, "out");
    assert!(!returns.is_generator);
    assert_eq!(parsed.examples, vec![">>> g(1)\n'out'"]);

    // A colon-terminated first token is a type, whatever the word.
    let odd = parse(
        "S.\n\nReturns:\n    Note: this is odd",
        DocstringStyle::Google,
    );
    let returns = odd.returns.unwrap();
    assert_eq!(returns.type_name.as_deref(), Some("Note"));
    assert_eq!(returns.description, "this is odd");

    // Attributes: items are params too; a later item with the same name wins downstream.
    let attrs = parse(
        "C.\n\nAttributes:\n    a (int): first\n    b: second",
        DocstringStyle::Google,
    );
    assert_eq!(attrs.params.len(), 2);
    assert_eq!(attrs.params[0].section, ParamSection::Attribute);
    assert_eq!(attrs.params[0].type_name.as_deref(), Some("int"));
}

#[test]
fn google_second_section_replaces_and_unknown_meta_truncates() {
    let text = "S.\n\nArgs:\n    a: one\n\nArgs:\n    b: two\n\nReturns:\n    r\nTrailing prose at column zero.\n    ignored";
    let parsed = parse(text, DocstringStyle::Google);
    assert_eq!(params_of(&parsed), vec![param("b", None, "two")]);
    assert_eq!(parsed.returns.unwrap().description, "r");

    // `Example:` and `Examples:` are different keys and both survive.
    let both = parse(
        "S.\n\nExample:\n    >>> a()\n\nExamples:\n    >>> b()",
        DocstringStyle::Google,
    );
    assert_eq!(both.examples, vec![">>> a()", ">>> b()"]);
}

#[test]
fn google_malformed_items_fall_back() {
    // Forced Google: the parser raises, Folioh splits first line / rest.
    let text = "Summary.\n\nMore.\n\nArgs:\n    x no colon here";
    let forced = parse(text, DocstringStyle::Google);
    assert_eq!(forced.short_description, "Summary.");
    assert_eq!(
        forced.long_description,
        "More.\n\nArgs:\n    x no colon here"
    );
    assert!(forced.params.is_empty());
    // Auto: Google fails, ReST wins with zero meta and the same visible result.
    let auto = parse(text, DocstringStyle::Auto);
    assert_eq!(auto.style, ParsedStyle::Rest);
    assert_eq!(auto.long_description, "More.\n\nArgs:\n    x no colon here");
    assert!(auto.params.is_empty());

    // An empty section body has no specification.
    let empty = parse("S.\n\nArgs:\n", DocstringStyle::Google);
    assert_eq!(empty.short_description, "S.");
    assert_eq!(empty.long_description, "Args:");
    // Raises without a colon.
    let raises = parse("S.\n\nRaises:\n    ValueError", DocstringStyle::Google);
    assert!(raises.raises.is_empty());
    assert_eq!(raises.long_description, "Raises:\n    ValueError");
    // A whitespace-only line right after the title does not hide the items.
    let blank_first = parse(
        "S.\n\nArgs:\n    \n    a: one\n    b: two",
        DocstringStyle::Google,
    );
    assert_eq!(
        params_of(&blank_first),
        vec![param("a", None, "one"), param("b", None, "two")]
    );
}

#[test]
fn plain_prose_and_empty_text() {
    let one = parse("Just a line.", DocstringStyle::Auto);
    assert_eq!(one.short_description, "Just a line.");
    assert_eq!(one.long_description, "");
    assert_eq!(one.style, ParsedStyle::Rest);
    let split = parse("S.\nnext line\n\nlong", DocstringStyle::Google);
    assert_eq!(split.short_description, "S.");
    assert_eq!(split.long_description, "next line\n\nlong");
    let empty = parse("", DocstringStyle::Auto);
    assert_eq!(to_docstring_ir(&empty), DocstringIR::default());
}

#[test]
fn numpy_docs_example() {
    for style in [DocstringStyle::Numpy, DocstringStyle::Auto] {
        let parsed = parse(NUMPY_DOC, style);
        assert_eq!(parsed.style, ParsedStyle::Numpy);
        assert_eq!(
            parsed.short_description,
            "Establish a connection to the remote server."
        );
        assert_eq!(parsed.long_description, "");
        assert_eq!(
            params_of(&parsed),
            vec![
                param("host", Some("str"), "The hostname or IP address."),
                param("port", Some("int"), "The port number, by default 8080."),
            ]
        );
        assert_eq!(parsed.params[0].is_optional, Some(false));
        assert_eq!(parsed.params[1].is_optional, Some(true));
        assert_eq!(parsed.params[1].default.as_deref(), Some("8080"));
        let returns = parsed.returns.as_ref().unwrap();
        assert_eq!(returns.type_name.as_deref(), Some("Connection"));
        assert_eq!(returns.return_name, None);
        assert_eq!(returns.description, "The active connection object.");
        assert_eq!(parsed.raises.len(), 1);
        assert_eq!(
            parsed.raises[0].type_name.as_deref(),
            Some("ConnectionError")
        );
        assert_eq!(
            parsed.raises[0].description,
            "If the server is unreachable."
        );
        // The snippet is kept beside the prose.
        assert_eq!(
            parsed.examples,
            vec![">>> conn = connect(\"localhost\")\n>>> conn.is_alive()\nTrue\nSome prose."]
        );
        assert_eq!(parsed.notes, vec!["Keeps the socket open."]);
    }
}

#[test]
fn numpy_keys_defaults_warns_and_dashes() {
    let text = "S.\n\nParameters\n----------\ncopy : bool, default True\n    Copy it.\nflag : bool, default=False\n    Flag.\nmode : str, default: 'r'\n    Mode.\nlevel\n    Defaults to 3.\nport : int\n    Use default=4 when unsure.\n\nOther Parameters\n----------------\nextra : int\n    More.\n\nReturns\n-------\nout : int\n    The out.\n\nWarns\n-----\nUserWarning\n    When odd.\n\nExamples\n--------\n>>> only_snippet()";
    let parsed = parse(text, DocstringStyle::Numpy);
    assert_eq!(
        params_of(&parsed),
        vec![
            param("copy", Some("bool"), "Copy it."),
            param("flag", Some("bool"), "Flag."),
            param("mode", Some("str"), "Mode."),
            param("level", None, "Defaults to 3."),
            param("port", Some("int"), "Use default=4 when unsure."),
            param("extra", Some("int"), "More."),
        ]
    );
    assert_eq!(
        parsed
            .params
            .iter()
            .map(|p| p.default.clone())
            .collect::<Vec<_>>(),
        vec![
            Some("True".to_string()),
            Some("False".to_string()),
            Some("'r'".to_string()),
            Some("3".to_string()),
            Some("4".to_string()),
            None,
        ]
    );
    assert_eq!(parsed.params[0].is_optional, Some(true));
    assert_eq!(parsed.params[5].section, ParamSection::OtherParam);
    let returns = parsed.returns.unwrap();
    assert_eq!(returns.return_name.as_deref(), Some("out"));
    assert_eq!(returns.type_name.as_deref(), Some("int"));
    assert_eq!(parsed.raises.len(), 1);
    assert_eq!(parsed.raises[0].type_name.as_deref(), Some("UserWarning"));
    assert_eq!(parsed.examples, vec![">>> only_snippet()"]);

    // The underline must be exactly as long as the title.
    let short_dashes = parse(
        "S.\n\nParameters\n---------\nx : int\n    X.",
        DocstringStyle::Numpy,
    );
    assert!(short_dashes.params.is_empty());
    assert_eq!(
        short_dashes.long_description,
        "Parameters\n---------\nx : int\n    X."
    );
}

#[test]
fn rest_fields() {
    let text = "Read a file.\n\n:param path: Location to read.\n:type path: str\n:param int count: How many.\n:param str? maybe: Optional one, defaults to 'a'.\n:returns: The file contents.\n:rtype: str\n:raises IOError: when bad\n:note: A rest note.\n:meta private:";
    let parsed = parse(text, DocstringStyle::Auto);
    assert_eq!(parsed.style, ParsedStyle::Rest);
    assert_eq!(parsed.short_description, "Read a file.");
    assert_eq!(parsed.long_description, "");
    assert_eq!(
        params_of(&parsed),
        vec![
            param("path", Some("str"), "Location to read."),
            param("count", Some("int"), "How many."),
            param("maybe", Some("str"), "Optional one, defaults to 'a'."),
        ]
    );
    assert_eq!(parsed.params[2].is_optional, Some(true));
    assert_eq!(parsed.params[2].default.as_deref(), Some("'a'"));
    let returns = parsed.returns.unwrap();
    assert_eq!(returns.type_name.as_deref(), Some("str"));
    assert_eq!(returns.description, "The file contents.");
    assert_eq!(parsed.raises.len(), 1);
    assert_eq!(parsed.raises[0].type_name.as_deref(), Some("IOError"));
    assert_eq!(parsed.raises[0].description, "when bad");
    assert_eq!(parsed.notes, vec!["A rest note."]);

    // `:rtype:` alone still yields a returns item.
    let rtype_only = parse("S.\n\n:rtype: int", DocstringStyle::Auto);
    let returns = rtype_only.returns.unwrap();
    assert_eq!(returns.type_name.as_deref(), Some("int"));
    assert_eq!(returns.description, "");

    // A field missing its second colon fails ReST; Google wins in auto.
    let broken = parse("S.\n\n:param x", DocstringStyle::Auto);
    assert_eq!(broken.style, ParsedStyle::Google);
    assert_eq!(broken.long_description, ":param x");
    // An empty field key degrades the same way instead of aborting the file.
    for text in ["S.\n\n: : y", "S.\n\n:  :"] {
        let empty_key = parse(text, DocstringStyle::Auto);
        assert_eq!(empty_key.short_description, "S.");
        assert_eq!(
            empty_key.long_description,
            text.trim_start_matches("S.\n\n")
        );
        assert!(empty_key.params.is_empty());
    }
}

#[test]
fn auto_ties_and_mixed_markers() {
    // NumPy 2 items (the `:param y:` line becomes a malformed param) beat ReST's 1.
    let mixed = "S.\n\nParameters\n----------\nx : int\n    X.\n:param y: b";
    let parsed = parse(mixed, DocstringStyle::Auto);
    assert_eq!(parsed.style, ParsedStyle::Numpy);
    assert_eq!(parsed.params.len(), 2);
    assert_eq!(parsed.params[1].arg_name, "");

    // Epydoc decides docstrings with `@word:` at column 0.
    let epy = "S.\n\n@param x: the x\n@type x: int\n@return: r\n@rtype: str\n@raise ValueError: bad\n@note: n";
    let parsed = parse(epy, DocstringStyle::Auto);
    assert_eq!(parsed.style, ParsedStyle::Epydoc);
    assert_eq!(params_of(&parsed), vec![param("x", Some("int"), "the x")]);
    let returns = parsed.returns.unwrap();
    assert_eq!(returns.type_name.as_deref(), Some("str"));
    assert_eq!(returns.description, "r");
    assert_eq!(parsed.raises[0].type_name.as_deref(), Some("ValueError"));
    assert_eq!(parsed.notes, vec!["n"]);

    // Google `Args:` beats a lone ReST field in auto.
    let google = "S.\n\nArgs:\n    a: one\n    b: two\n\n:note: tail";
    assert_eq!(
        parse(google, DocstringStyle::Auto).style,
        ParsedStyle::Google
    );
}

#[test]
fn splitlines_matches_python() {
    assert_eq!(splitlines("a\nb\r\nc\rd"), vec!["a", "b", "c", "d"]);
    assert_eq!(splitlines("a\n"), vec!["a"]);
    assert_eq!(splitlines(""), Vec::<&str>::new());
    assert_eq!(splitlines("a\x0cb\u{2028}c"), vec!["a", "b", "c"]);
}

#[test]
fn prose_sections_close_the_long_description_in_every_style() {
    let google = parse(
        "S.\n\nBody.\n\nArgs:\n    a: one\n\nWarning:\n    Slow on large input.\n\nSee Also:\n    other_function\n\nTodo:\n    Cache it.\n\nDeprecated:\n    Use v2.",
        DocstringStyle::Google,
    );
    assert_eq!(params_of(&google), vec![param("a", None, "one")]);
    assert_eq!(
        google.long_description,
        "Body.\n\n**Warning:** Slow on large input.\n\n**See also:** other_function\n\n**Todo:** Cache it.\n\n**Deprecated:** Use v2."
    );

    let numpy = parse(
        "S.\n\n.. deprecated:: 1.6.0\n    Use `other` instead.\n\nParameters\n----------\nx : int\n    X.\n\nWarnings\n--------\nNot thread safe.\n\nSee Also\n--------\nother : Does the same.",
        DocstringStyle::Numpy,
    );
    assert_eq!(
        numpy.long_description,
        ".. deprecated:: 1.6.0\n    Use `other` instead.\n\n**Warning:** Not thread safe.\n\n**See also:** other : Does the same."
    );

    let rest = parse(
        "S.\n\n:param x: X.\n:warning: Loud.\n:meta private:",
        DocstringStyle::Auto,
    );
    assert_eq!(rest.long_description, "**Warning:** Loud.");
}

#[test]
fn sphinx_roles_read_as_code_or_text_in_every_style() {
    assert_eq!(
        plain_roles("Returns a :class:`Widget`, see :py:meth:`~pkg.Widget.render` and :ref:`the guide <guide>`."),
        "Returns a `Widget`, see `render` and the guide."
    );
    assert_eq!(
        plain_roles("A ratio 1:`x` and ``code``."),
        "A ratio 1:`x` and ``code``."
    );
    let parsed = parse(
        "Build a :class:`Box`.\n\nArgs:\n    size (int): Passed to :func:`make`.\n\nRaises:\n    ValueError: When :attr:`Box.size` is negative.\n",
        DocstringStyle::Google,
    );
    assert_eq!(parsed.short_description, "Build a `Box`.");
    assert_eq!(parsed.params[0].description, "Passed to `make`.");
    assert_eq!(parsed.raises[0].description, "When `Box.size` is negative.");
}
