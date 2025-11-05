// This module is kept for potential future use but currently unused
#![allow(dead_code)]

use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::rc::Rc;

use super::value::Value;

#[derive(Clone, Debug)]
pub struct CliParseOutcome {
    pub ok: bool,
    pub exit: i64,
    pub command: String,
    pub path: Vec<String>,
    pub scopes: Vec<CliScopeOutcome>,
    pub options: HashMap<String, Value>,
    pub positionals: HashMap<String, Value>,
    pub rest: Vec<String>,
    pub message: String,
    pub help: String,
}

#[derive(Clone, Debug)]
pub struct CliScopeOutcome {
    pub name: String,
    pub options: HashMap<String, Value>,
    pub positionals: HashMap<String, Value>,
}

pub fn parse_cli(
    spec_value: &Value,
    args: &[String],
    program_name: Option<&str>,
) -> Result<CliParseOutcome> {
    let mut spec = parse_command_spec(spec_value, program_name)?;
    if spec.name.is_empty() {
        spec.name = program_name.unwrap_or("tea").to_string();
    }
    let path = vec![spec.name.clone()];
    match parse_command(&spec, args, path)? {
        ParseFlow::Success(mut success) => {
            let help = render_help(success.help_spec, &success.path);
            success.outcome.help = help.clone();
            success.outcome.message = String::new();
            success.outcome.help = help;
            Ok(success.outcome)
        }
        ParseFlow::Help(help) => Ok(CliParseOutcome {
            ok: false,
            exit: 0,
            command: help.command,
            path: help.path,
            scopes: help.scopes,
            options: HashMap::new(),
            positionals: HashMap::new(),
            rest: Vec::new(),
            message: help.help.clone(),
            help: help.help,
        }),
        ParseFlow::Error(error) => Ok(CliParseOutcome {
            ok: false,
            exit: 64,
            command: error.command,
            path: error.path,
            scopes: error.scopes,
            options: HashMap::new(),
            positionals: HashMap::new(),
            rest: Vec::new(),
            message: error.message,
            help: error.help,
        }),
    }
}

#[derive(Clone, Debug)]
struct CommandSpec {
    name: String,
    description: Option<String>,
    version: Option<String>,
    allow_trailing: bool,
    default_command: Option<String>,
    options: Vec<OptionSpec>,
    positionals: Vec<PositionalSpec>,
    subcommands: Vec<CommandSpec>,
    switch_map: HashMap<String, usize>,
    subcommand_map: HashMap<String, usize>,
}

#[derive(Clone, Debug)]
struct OptionSpec {
    name: String,
    switches: Vec<String>,
    description: Option<String>,
    kind: OptionKind,
    value_type: ValueType,
    required: bool,
    multiple: bool,
    default: OptionDefault,
}

#[derive(Clone, Debug)]
enum OptionKind {
    Flag,
    Value,
}

#[derive(Clone, Debug)]
enum OptionDefault {
    None,
    Bool(bool),
    Single(Value),
    Many(Vec<Value>),
}

#[derive(Clone, Debug)]
struct PositionalSpec {
    name: String,
    description: Option<String>,
    value_type: ValueType,
    required: bool,
    multiple: bool,
}

#[derive(Clone, Debug)]
enum ValueType {
    String,
    Int,
    Float,
    Bool,
}

#[derive(Clone, Debug)]
struct OptionParseState<'a> {
    spec: &'a OptionSpec,
    value: OptionValueState,
}

#[derive(Clone, Debug)]
enum OptionValueState {
    Flag {
        value: bool,
    },
    Single {
        value: Option<Value>,
        provided: bool,
    },
    Multi {
        values: Vec<Value>,
        provided: bool,
    },
}

enum ParseFlow<'a> {
    Success(ParseSuccess<'a>),
    Help(ParseHelp),
    Error(ParseError),
}

struct ParseSuccess<'a> {
    outcome: CliParseOutcome,
    help_spec: &'a CommandSpec,
    path: Vec<String>,
}

struct ParseHelp {
    command: String,
    path: Vec<String>,
    scopes: Vec<CliScopeOutcome>,
    help: String,
}

struct ParseError {
    command: String,
    path: Vec<String>,
    scopes: Vec<CliScopeOutcome>,
    message: String,
    help: String,
}

fn parse_command_spec(value: &Value, program_name: Option<&str>) -> Result<CommandSpec> {
    let map = expect_dict(value, "command spec")?;
    let mut name = get_string(map, "name")?;
    if name.is_empty() {
        if let Some(program) = program_name {
            name = program.to_string();
        }
    }
    if name.is_empty() {
        bail!("command spec requires a non-empty 'name'");
    }

    let description = get_optional_string(map, "description")?;
    let version = get_optional_string(map, "version")?;
    let allow_trailing = get_bool(map, "allow_trailing", true)?;
    let default_command = get_optional_string(map, "default_command")?;

    let options = map
        .get("options")
        .map(|value| {
            expect_list(value, "options").and_then(|items| {
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| {
                        parse_option_spec(&item)
                            .with_context(|| format!("while parsing option {}", index + 1))
                    })
                    .collect::<Result<Vec<_>>>()
            })
        })
        .transpose()?
        .unwrap_or_default();

    let positionals = map
        .get("positionals")
        .map(|value| {
            expect_list(value, "positionals").and_then(|items| {
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| {
                        parse_positional_spec(&item)
                            .with_context(|| format!("while parsing positional {}", index + 1))
                    })
                    .collect::<Result<Vec<_>>>()
            })
        })
        .transpose()?
        .unwrap_or_default();

    validate_positionals(&positionals)?;

    let subcommands = map
        .get("subcommands")
        .map(|value| {
            expect_list(value, "subcommands").and_then(|items| {
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| {
                        parse_command_spec(&item, None)
                            .with_context(|| format!("while parsing subcommand {}", index + 1))
                    })
                    .collect::<Result<Vec<_>>>()
            })
        })
        .transpose()?
        .unwrap_or_default();

    let mut switch_map = HashMap::new();
    for (index, option) in options.iter().enumerate() {
        for switch in &option.switches {
            if switch == "-h" || switch == "--help" {
                bail!("switch '{}' is reserved for help output", switch);
            }
            if switch_map.insert(switch.clone(), index).is_some() {
                bail!("duplicate switch definition '{}'", switch);
            }
        }
    }

    let mut subcommand_map = HashMap::new();
    for (index, command) in subcommands.iter().enumerate() {
        if subcommand_map.insert(command.name.clone(), index).is_some() {
            bail!(format!(
                "duplicate subcommand definition '{}'",
                command.name
            ));
        }
    }

    if let Some(default) = &default_command {
        if !subcommand_map.contains_key(default) {
            bail!(format!(
                "default_command '{}' does not match any subcommand",
                default
            ));
        }
    }

    Ok(CommandSpec {
        name,
        description,
        version,
        allow_trailing,
        default_command,
        options,
        positionals,
        subcommands,
        switch_map,
        subcommand_map,
    })
}

fn parse_option_spec(value: &Value) -> Result<OptionSpec> {
    let map = expect_dict(value, "option spec")?;
    let name = get_string(map, "name")?;
    if name.is_empty() {
        bail!("option spec requires a non-empty 'name'");
    }
    let switches = get_string_list(map, "aliases")?;
    if switches.is_empty() {
        bail!("option '{}' requires at least one alias", name);
    }
    for switch in &switches {
        if !switch.starts_with('-') {
            bail!(format!(
                "option '{}' alias '{}' must begin with '-'",
                name, switch
            ));
        }
    }
    let description = get_optional_string(map, "description")?;
    let kind_str = get_string(map, "kind")?.to_ascii_lowercase();
    let kind = match kind_str.as_str() {
        "flag" => OptionKind::Flag,
        "option" => OptionKind::Value,
        other => bail!("unknown option kind '{}'", other),
    };
    let value_type = if let OptionKind::Value = kind {
        parse_value_type(map, "type")?
    } else {
        ValueType::Bool
    };
    let required = get_bool(map, "required", false)?;
    let multiple = get_bool(map, "multiple", false)?;
    if matches!(kind, OptionKind::Flag) && multiple {
        bail!("flag option '{}' cannot set multiple=true", name);
    }
    let default = if let Some(default_value) = map.get("default") {
        if default_value.is_nil() {
            OptionDefault::None
        } else {
            match kind {
                OptionKind::Flag => match default_value {
                    Value::Bool(flag) => OptionDefault::Bool(*flag),
                    _ => bail!(format!("flag option '{}' default must be Bool", name)),
                },
                OptionKind::Value => {
                    if multiple {
                        match default_value {
                            Value::List(items) => OptionDefault::Many(items.as_ref().clone()),
                            Value::Nil => OptionDefault::Many(Vec::new()),
                            _ => bail!(format!(
                                "option '{}' expects default to be a List when multiple=true",
                                name
                            )),
                        }
                    } else {
                        let coerced = coerce_value(default_value, &value_type)
                            .with_context(|| format!("invalid default for option '{}'", name))?;
                        OptionDefault::Single(coerced)
                    }
                }
            }
        }
    } else {
        OptionDefault::None
    };

    Ok(OptionSpec {
        name,
        switches,
        description,
        kind,
        value_type,
        required,
        multiple,
        default,
    })
}

fn parse_positional_spec(value: &Value) -> Result<PositionalSpec> {
    let map = expect_dict(value, "positional spec")?;
    let name = get_string(map, "name")?;
    if name.is_empty() {
        bail!("positional spec requires a non-empty 'name'");
    }
    let description = get_optional_string(map, "description")?;
    let value_type = parse_value_type(map, "type")?;
    let required = get_bool(map, "required", true)?;
    let multiple = get_bool(map, "multiple", false)?;
    Ok(PositionalSpec {
        name,
        description,
        value_type,
        required,
        multiple,
    })
}

fn validate_positionals(positionals: &[PositionalSpec]) -> Result<()> {
    if let Some((index, _)) = positionals
        .iter()
        .enumerate()
        .filter(|(_, positional)| positional.multiple)
        .find(|(index, _)| *index != positionals.len() - 1)
    {
        bail!(format!(
            "positional {} sets multiple=true but is not the final positional",
            index + 1
        ));
    }

    let mut optional_seen = false;
    for (index, positional) in positionals.iter().enumerate() {
        if !positional.required {
            optional_seen = true;
        } else if optional_seen {
            bail!(format!(
                "positional {} is required but follows an optional positional",
                index + 1
            ));
        }
    }
    Ok(())
}

fn parse_command<'a>(
    spec: &'a CommandSpec,
    args: &[String],
    path: Vec<String>,
) -> Result<ParseFlow<'a>> {
    let mut option_states = initialise_option_state(spec)?;
    let mut consumed = 0usize;
    while consumed < args.len() {
        let token = &args[consumed];
        if token == "--" {
            consumed += 1;
            break;
        }
        if token == "--help" {
            let scope = current_scope(spec, &option_states, HashMap::new())?;
            let help = render_help(spec, &path);
            return Ok(ParseFlow::Help(ParseHelp {
                command: spec.name.clone(),
                path,
                scopes: vec![scope],
                help,
            }));
        }
        if token.starts_with("--") {
            let (switch, inline) = if let Some((name, value)) = token.split_once('=') {
                (name.to_string(), Some(value.to_string()))
            } else {
                (token.clone(), None)
            };
            if switch == "--help" {
                let scope = current_scope(spec, &option_states, HashMap::new())?;
                let help = render_help(spec, &path);
                return Ok(ParseFlow::Help(ParseHelp {
                    command: spec.name.clone(),
                    path,
                    scopes: vec![scope],
                    help,
                }));
            }
            if let Some(index) = spec.switch_map.get(&switch) {
                match handle_option(
                    &mut option_states[*index],
                    inline.clone(),
                    if inline.is_none() {
                        args.get(consumed + 1)
                    } else {
                        None
                    },
                    &switch,
                )? {
                    OptionConsumption::Inline => consumed += 1,
                    OptionConsumption::Next => {
                        consumed += 2;
                        if inline.is_none() && args.get(consumed - 1).is_none() {
                            return usage_error(
                                spec,
                                path,
                                option_states,
                                HashMap::new(),
                                format!("option '{}' requires a value", switch),
                            );
                        }
                    }
                    OptionConsumption::None => consumed += 1,
                }
                continue;
            } else {
                let scope = current_scope(spec, &option_states, HashMap::new())?;
                let help = render_help(spec, &path);
                return Ok(ParseFlow::Error(ParseError {
                    command: spec.name.clone(),
                    path,
                    scopes: vec![scope],
                    message: format!("unknown option '{}'", switch),
                    help,
                }));
            }
        } else if token.starts_with('-') && token.len() > 1 {
            if token == "-h" {
                let scope = current_scope(spec, &option_states, HashMap::new())?;
                let help = render_help(spec, &path);
                return Ok(ParseFlow::Help(ParseHelp {
                    command: spec.name.clone(),
                    path,
                    scopes: vec![scope],
                    help,
                }));
            }
            if token.len() != 2 {
                return usage_error(
                    spec,
                    path,
                    option_states,
                    HashMap::new(),
                    format!("combined short options '{}' are not supported", token),
                );
            }
            if let Some(index) = spec.switch_map.get(token) {
                match handle_option(
                    &mut option_states[*index],
                    None,
                    args.get(consumed + 1),
                    token,
                )? {
                    OptionConsumption::Inline => consumed += 1,
                    OptionConsumption::Next => {
                        if args.get(consumed + 1).is_none() {
                            return usage_error(
                                spec,
                                path,
                                option_states,
                                HashMap::new(),
                                format!("option '{}' requires a value", token),
                            );
                        }
                        consumed += 2;
                    }
                    OptionConsumption::None => consumed += 1,
                }
                continue;
            } else {
                let scope = current_scope(spec, &option_states, HashMap::new())?;
                let help = render_help(spec, &path);
                return Ok(ParseFlow::Error(ParseError {
                    command: spec.name.clone(),
                    path,
                    scopes: vec![scope],
                    message: format!("unknown option '{}'", token),
                    help,
                }));
            }
        } else {
            break;
        }
    }

    let remaining = &args[consumed..];
    if let Some(subcommand_name) = remaining.first() {
        if let Some(index) = spec.subcommand_map.get(subcommand_name) {
            check_required_options(&option_states)?;
            if !spec.positionals.is_empty() {
                return usage_error(
                    spec,
                    path,
                    option_states,
                    HashMap::new(),
                    "subcommand provided but parent command defines positionals".to_string(),
                );
            }
            let current_scope = current_scope(spec, &option_states, HashMap::new())?;
            let mut new_path = path.clone();
            new_path.push(spec.subcommands[*index].name.clone());
            return match parse_command(&spec.subcommands[*index], &remaining[1..], new_path)? {
                ParseFlow::Success(mut success) => {
                    success.outcome.scopes.insert(0, current_scope);
                    Ok(ParseFlow::Success(success))
                }
                ParseFlow::Help(mut help) => {
                    help.scopes.insert(0, current_scope);
                    Ok(ParseFlow::Help(help))
                }
                ParseFlow::Error(mut error) => {
                    error.scopes.insert(0, current_scope);
                    Ok(ParseFlow::Error(error))
                }
            };
        } else if !spec.subcommands.is_empty() && spec.default_command.is_some() {
            // fall through to default handling below
        } else if !spec.subcommands.is_empty() {
            let scope = current_scope(spec, &option_states, HashMap::new())?;
            let help = render_help(spec, &path);
            return Ok(ParseFlow::Error(ParseError {
                command: spec.name.clone(),
                path,
                scopes: vec![scope],
                message: format!("unknown subcommand '{}'", subcommand_name),
                help,
            }));
        }
    }

    if remaining.is_empty() {
        if let Some(default_name) = &spec.default_command {
            if let Some(index) = spec.subcommand_map.get(default_name) {
                check_required_options(&option_states)?;
                let current_scope = current_scope(spec, &option_states, HashMap::new())?;
                let mut new_path = path.clone();
                new_path.push(spec.subcommands[*index].name.clone());
                return match parse_command(&spec.subcommands[*index], remaining, new_path)? {
                    ParseFlow::Success(mut success) => {
                        success.outcome.scopes.insert(0, current_scope);
                        Ok(ParseFlow::Success(success))
                    }
                    ParseFlow::Help(mut help) => {
                        help.scopes.insert(0, current_scope);
                        Ok(ParseFlow::Help(help))
                    }
                    ParseFlow::Error(mut error) => {
                        error.scopes.insert(0, current_scope);
                        Ok(ParseFlow::Error(error))
                    }
                };
            } else {
                bail!("internal error: default command '{}' missing", default_name);
            }
        }
    }

    check_required_options(&option_states)?;
    let (positionals_map, consumed_positionals) =
        match parse_positionals(&spec.positionals, remaining) {
            Ok(values) => values,
            Err(message) => {
                return usage_error(
                    spec,
                    path.clone(),
                    option_states.clone(),
                    HashMap::new(),
                    message,
                );
            }
        };
    let rest: Vec<String> = remaining[consumed_positionals..].to_vec();
    if !spec.allow_trailing && !rest.is_empty() {
        return usage_error(
            spec,
            path,
            option_states,
            HashMap::new(),
            format!("unexpected arguments: {}", rest.join(" ")),
        );
    }

    let options_map = collect_option_values(&option_states);
    let scope = CliScopeOutcome {
        name: spec.name.clone(),
        options: options_map.clone(),
        positionals: positionals_map.clone(),
    };

    let outcome = CliParseOutcome {
        ok: true,
        exit: 0,
        command: spec.name.clone(),
        path: path.clone(),
        scopes: vec![scope],
        options: options_map,
        positionals: positionals_map,
        rest,
        message: String::new(),
        help: String::new(),
    };

    Ok(ParseFlow::Success(ParseSuccess {
        outcome,
        help_spec: spec,
        path,
    }))
}

fn initialise_option_state(spec: &CommandSpec) -> Result<Vec<OptionParseState<'_>>> {
    spec.options
        .iter()
        .map(|option| {
            let value = match option.kind {
                OptionKind::Flag => {
                    let default = match &option.default {
                        OptionDefault::Bool(value) => *value,
                        OptionDefault::None => false,
                        OptionDefault::Single(value) => match value {
                            Value::Bool(flag) => *flag,
                            _ => {
                                bail!(format!(
                                    "flag option '{}' default must be Bool",
                                    option.name
                                ))
                            }
                        },
                        OptionDefault::Many(_) => {
                            bail!(format!(
                                "flag option '{}' does not support list defaults",
                                option.name
                            ))
                        }
                    };
                    OptionValueState::Flag { value: default }
                }
                OptionKind::Value => {
                    if option.multiple {
                        let (values, provided) = match &option.default {
                            OptionDefault::None => (Vec::new(), false),
                            OptionDefault::Many(values) => (values.clone(), !values.is_empty()),
                            OptionDefault::Single(_) => {
                                bail!(format!(
                                    "option '{}' default must be a list when multiple=true",
                                    option.name
                                ))
                            }
                            OptionDefault::Bool(_) => {
                                bail!(format!(
                                    "option '{}' expects value defaults, not Bool",
                                    option.name
                                ))
                            }
                        };
                        OptionValueState::Multi { values, provided }
                    } else {
                        match &option.default {
                            OptionDefault::None => OptionValueState::Single {
                                value: None,
                                provided: false,
                            },
                            OptionDefault::Single(value) => OptionValueState::Single {
                                value: Some(value.clone()),
                                provided: true,
                            },
                            OptionDefault::Many(_) => {
                                bail!(format!(
                                    "option '{}' defaults to a list but multiple=false",
                                    option.name
                                ))
                            }
                            OptionDefault::Bool(_) => {
                                bail!(format!(
                                    "option '{}' expects a value default, not Bool",
                                    option.name
                                ))
                            }
                        }
                    }
                }
            };
            Ok(OptionParseState {
                spec: option,
                value,
            })
        })
        .collect()
}

enum OptionConsumption {
    None,
    Inline,
    Next,
}

fn handle_option(
    state: &mut OptionParseState,
    inline: Option<String>,
    lookahead: Option<&String>,
    switch: &str,
) -> Result<OptionConsumption> {
    match (&state.spec.kind, &mut state.value) {
        (OptionKind::Flag, OptionValueState::Flag { value }) => {
            *value = true;
            Ok(OptionConsumption::None)
        }
        (OptionKind::Value, OptionValueState::Single { value, provided }) => {
            let raw = if let Some(ref inline_value) = inline {
                inline_value.clone()
            } else if let Some(next) = lookahead {
                next.clone()
            } else {
                bail!("option '{}' expects a value", switch);
            };
            let coerced = coerce_value(&Value::String(raw.clone()), &state.spec.value_type)
                .with_context(|| format!("invalid value for option '{}'", switch))?;
            *value = Some(coerced);
            *provided = true;
            if inline.is_some() {
                Ok(OptionConsumption::Inline)
            } else {
                Ok(OptionConsumption::Next)
            }
        }
        (OptionKind::Value, OptionValueState::Multi { values, provided }) => {
            let raw = if let Some(ref inline_value) = inline {
                inline_value.clone()
            } else if let Some(next) = lookahead {
                next.clone()
            } else {
                bail!("option '{}' expects a value", switch);
            };
            let coerced = coerce_value(&Value::String(raw.clone()), &state.spec.value_type)
                .with_context(|| format!("invalid value for option '{}'", switch))?;
            values.push(coerced);
            *provided = true;
            if inline.is_some() {
                Ok(OptionConsumption::Inline)
            } else {
                Ok(OptionConsumption::Next)
            }
        }
        (OptionKind::Flag, OptionValueState::Single { .. })
        | (OptionKind::Flag, OptionValueState::Multi { .. })
        | (OptionKind::Value, OptionValueState::Flag { .. }) => unreachable!(),
    }
}

fn check_required_options(states: &[OptionParseState]) -> Result<()> {
    for state in states {
        if !state.spec.required {
            continue;
        }
        match &state.value {
            OptionValueState::Flag { value } => {
                if !value {
                    bail!(format!("missing required flag '{}'", state.spec.name));
                }
            }
            OptionValueState::Single { provided, .. } => {
                if !provided {
                    bail!(format!("missing required option '{}'", state.spec.name));
                }
            }
            OptionValueState::Multi { provided, .. } => {
                if !provided {
                    bail!(format!("missing required option '{}'", state.spec.name));
                }
            }
        }
    }
    Ok(())
}

fn collect_option_values(states: &[OptionParseState]) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    for state in states {
        let value = match &state.value {
            OptionValueState::Flag { value } => Value::Bool(*value),
            OptionValueState::Single { value, .. } => value.clone().unwrap_or(Value::Nil),
            OptionValueState::Multi { values, .. } => Value::List(Rc::new(values.clone())),
        };
        map.insert(state.spec.name.clone(), value);
    }
    map
}

fn parse_positionals(
    specs: &[PositionalSpec],
    args: &[String],
) -> Result<(HashMap<String, Value>, usize), String> {
    let mut consumed = 0usize;
    let mut result = HashMap::new();

    for positional in specs {
        if positional.multiple {
            let mut values: Vec<Value> = Vec::new();
            while consumed < args.len() {
                let token = &args[consumed];
                let value =
                    coerce_string(token.clone(), &positional.value_type).map_err(|err| {
                        format!(
                            "positional '{}' expects {}: {}",
                            positional.name,
                            describe_type(&positional.value_type),
                            err
                        )
                    })?;
                values.push(value);
                consumed += 1;
            }
            result.insert(positional.name.clone(), Value::List(Rc::new(values)));
        } else if consumed < args.len() {
            let token = &args[consumed];
            let value = coerce_string(token.clone(), &positional.value_type).map_err(|err| {
                format!(
                    "positional '{}' expects {}: {}",
                    positional.name,
                    describe_type(&positional.value_type),
                    err
                )
            })?;
            result.insert(positional.name.clone(), value);
            consumed += 1;
        } else if positional.required {
            return Err(format!(
                "missing required positional argument '{}'",
                positional.name
            ));
        } else {
            result.insert(positional.name.clone(), Value::Nil);
        }

        if positional.multiple {
            break;
        }
    }

    Ok((result, consumed))
}

fn current_scope(
    spec: &CommandSpec,
    states: &[OptionParseState],
    positionals: HashMap<String, Value>,
) -> Result<CliScopeOutcome> {
    let options = collect_option_values(states);
    Ok(CliScopeOutcome {
        name: spec.name.clone(),
        options,
        positionals,
    })
}

fn usage_error<'a>(
    spec: &'a CommandSpec,
    path: Vec<String>,
    states: Vec<OptionParseState<'a>>,
    positionals: HashMap<String, Value>,
    message: String,
) -> Result<ParseFlow<'a>> {
    let scope = CliScopeOutcome {
        name: spec.name.clone(),
        options: collect_option_values(&states),
        positionals,
    };
    let help = render_help(spec, &path);
    Ok(ParseFlow::Error(ParseError {
        command: spec.name.clone(),
        path,
        scopes: vec![scope],
        message,
        help,
    }))
}

fn render_help(spec: &CommandSpec, path: &[String]) -> String {
    let mut lines = Vec::new();
    let usage = build_usage(spec, path);
    lines.push(format!("Usage: {}", usage));
    if let Some(version) = &spec.version {
        lines.push(String::new());
        lines.push(format!("Version: {}", version));
    }
    if let Some(description) = &spec.description {
        if !description.trim().is_empty() {
            lines.push(String::new());
            lines.push(description.clone());
        }
    }

    let mut option_entries = Vec::new();
    if !spec.options.is_empty() {
        option_entries.push(format_option_usage(
            &["-h", "--help"],
            "Show this help message.",
        ));
        for option in &spec.options {
            let switches = option
                .switches
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>();
            let description = option
                .description
                .clone()
                .unwrap_or_else(|| String::from(""));
            let mut entry = format_option_usage(&switches, &description);
            if option.required {
                entry.push_str(" (required)");
            } else if let OptionDefault::Single(value) = &option.default {
                entry.push_str(&format!(" (default: {})", value));
            } else if let OptionDefault::Bool(value) = &option.default {
                entry.push_str(&format!(" (default: {})", value));
            } else if let OptionDefault::Many(values) = &option.default {
                if !values.is_empty() {
                    entry.push_str(&format!(
                        " (default: [{}])",
                        values
                            .iter()
                            .map(|value| value.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            if option.multiple {
                entry.push_str(" (repeatable)");
            }
            option_entries.push(entry);
        }
    }

    if !option_entries.is_empty() {
        lines.push(String::new());
        lines.push("Options:".to_string());
        lines.extend(option_entries);
    }

    if !spec.positionals.is_empty() {
        lines.push(String::new());
        lines.push("Positionals:".to_string());
        for positional in &spec.positionals {
            let mut entry = format!("  {}", positional.name);
            if !positional.required {
                entry.push_str(" (optional)");
            }
            if positional.multiple {
                entry.push_str("...");
            }
            if let Some(description) = &positional.description {
                if !description.trim().is_empty() {
                    entry.push_str(&format!(" - {}", description));
                }
            }
            lines.push(entry);
        }
    }

    if !spec.subcommands.is_empty() {
        lines.push(String::new());
        lines.push("Subcommands:".to_string());
        for sub in &spec.subcommands {
            if let Some(description) = &sub.description {
                lines.push(format!("  {:15} {}", sub.name, description));
            } else {
                lines.push(format!("  {}", sub.name));
            }
        }
    }

    lines.join("\n")
}

fn build_usage(spec: &CommandSpec, path: &[String]) -> String {
    let mut usage = path.join(" ");
    if !spec.options.is_empty() {
        usage.push_str(" [OPTIONS]");
    }
    for positional in &spec.positionals {
        if positional.required {
            usage.push(' ');
            usage.push_str(&positional.name.to_uppercase());
        } else {
            usage.push(' ');
            usage.push_str(&format!("[{}]", positional.name.to_uppercase()));
        }
        if positional.multiple {
            usage.push_str("...");
        }
    }
    if !spec.subcommands.is_empty() {
        usage.push_str(" <SUBCOMMAND>");
    }
    usage
}

fn format_option_usage(switches: &[&str], description: &str) -> String {
    let label = switches.join(", ");
    if description.trim().is_empty() {
        format!("  {}", label)
    } else {
        format!("  {:20} {}", label, description)
    }
}

fn get_string(map: &HashMap<String, Value>, key: &str) -> Result<String> {
    match map.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(Value::Nil) | None => Ok(String::new()),
        _ => bail!("field '{}' must be a String", key),
    }
}

fn get_optional_string(map: &HashMap<String, Value>, key: &str) -> Result<Option<String>> {
    match map.get(key) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Nil) | None => Ok(None),
        _ => bail!("field '{}' must be a String", key),
    }
}

fn get_bool(map: &HashMap<String, Value>, key: &str, default: bool) -> Result<bool> {
    match map.get(key) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(Value::Nil) | None => Ok(default),
        _ => bail!("field '{}' must be a Bool", key),
    }
}

fn get_string_list(map: &HashMap<String, Value>, key: &str) -> Result<Vec<String>> {
    match map.get(key) {
        Some(Value::List(items)) => {
            let mut result = Vec::with_capacity(items.len());
            for item in items.iter() {
                match item {
                    Value::String(value) => result.push(value.clone()),
                    _ => bail!("field '{}' must be a list of Strings", key),
                }
            }
            Ok(result)
        }
        Some(Value::Nil) | None => Ok(Vec::new()),
        _ => bail!("field '{}' must be a List", key),
    }
}

fn parse_value_type(map: &HashMap<String, Value>, key: &str) -> Result<ValueType> {
    match map.get(key) {
        Some(Value::String(value)) => match value.to_ascii_lowercase().as_str() {
            "string" => Ok(ValueType::String),
            "int" | "integer" => Ok(ValueType::Int),
            "float" | "double" => Ok(ValueType::Float),
            "bool" | "boolean" => Ok(ValueType::Bool),
            other => bail!(format!("unknown value type '{}'", other)),
        },
        Some(Value::Nil) | None => Ok(ValueType::String),
        _ => bail!(format!("field '{}' must be a String type name", key)),
    }
}

fn expect_dict<'a>(value: &'a Value, context: &str) -> Result<&'a HashMap<String, Value>> {
    match value {
        Value::Dict(entries) => Ok(entries),
        _ => bail!("expected {} to be a dictionary", context),
    }
}

fn expect_list(value: &Value, context: &str) -> Result<Vec<Value>> {
    match value {
        Value::List(items) => Ok(items.as_ref().clone()),
        _ => bail!("expected {} to be a list", context),
    }
}

fn coerce_value(value: &Value, ty: &ValueType) -> Result<Value> {
    match (ty, value) {
        (ValueType::String, Value::String(_)) => Ok(value.clone()),
        (ValueType::String, other) => Ok(Value::String(other.to_string())),
        (ValueType::Int, Value::Int(_))
        | (ValueType::Float, Value::Float(_))
        | (ValueType::Bool, Value::Bool(_)) => Ok(value.clone()),
        (ValueType::Int, Value::String(text)) => {
            let parsed: i64 = text
                .parse()
                .map_err(|err| anyhow!("failed to parse int: {}", err))?;
            Ok(Value::Int(parsed))
        }
        (ValueType::Float, Value::String(text)) => {
            let parsed: f64 = text
                .parse()
                .map_err(|err| anyhow!("failed to parse float: {}", err))?;
            Ok(Value::Float(parsed))
        }
        (ValueType::Bool, Value::String(text)) => {
            let lower = text.to_ascii_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "y" => Ok(Value::Bool(true)),
                "false" | "0" | "no" | "n" => Ok(Value::Bool(false)),
                _ => bail!("failed to parse bool"),
            }
        }
        (ValueType::Int, Value::Float(value)) => Ok(Value::Int(*value as i64)),
        (ValueType::Float, Value::Int(value)) => Ok(Value::Float(*value as f64)),
        (_, Value::Nil) => Ok(Value::Nil),
        _ => bail!("unsupported default value type"),
    }
}

fn coerce_string(raw: String, ty: &ValueType) -> Result<Value, String> {
    match ty {
        ValueType::String => Ok(Value::String(raw)),
        ValueType::Int => raw
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|err| err.to_string()),
        ValueType::Float => raw
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|err| err.to_string()),
        ValueType::Bool => {
            let lower = raw.to_ascii_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "y" => Ok(Value::Bool(true)),
                "false" | "0" | "no" | "n" => Ok(Value::Bool(false)),
                _ => Err(format!("invalid boolean '{}'", raw)),
            }
        }
    }
}

fn describe_type(ty: &ValueType) -> &'static str {
    match ty {
        ValueType::String => "a String",
        ValueType::Int => "an Int",
        ValueType::Float => "a Float",
        ValueType::Bool => "a Bool",
    }
}

trait ValueExt {
    fn is_nil(&self) -> bool;
}

impl ValueExt for Value {
    fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
}
