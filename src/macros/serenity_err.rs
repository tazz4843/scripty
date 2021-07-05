#[macro_export]
macro_rules! handle_serenity_error {
    ($e:expr) => {
        {
            use std::io::ErrorKind as IoErrorKind;
            use std::num::IntErrorKind;
            use serde_json::error::Category;
            use serenity::Error;
            use serenity::{
                http::error::Error as HttpError,
                model::error::Error as ModelError,
                prelude::{ClientError, GatewayError},
            };
            match $e {
                Error::Decode(reason, _) => {
                    format!(
                        "Something went wrong while decoding Discord's response... try again? {}",
                        reason
                    )
                }
                Error::Format(err) => {
                    format!(
                        "Something went wrong while formatting a response... try again? {}",
                        err
                    )
                }
                Error::Io(err) => {
                    let err_msg1 = match err.kind() {
                        IoErrorKind::NotFound => "Resource not found".to_string(),
                        IoErrorKind::PermissionDenied => {
                            "Permission denied for resource".to_string()
                        }
                        IoErrorKind::ConnectionRefused => {
                            "Remote server refused connection".to_string()
                        }
                        IoErrorKind::ConnectionReset => {
                            "Remote server reset connection".to_string()
                        }
                        IoErrorKind::ConnectionAborted => {
                            "Remote server aborted connection".to_string()
                        }
                        IoErrorKind::NotConnected => "Network not connected yet".to_string(),
                        IoErrorKind::AddrInUse => "Address already in use".to_string(),
                        IoErrorKind::AddrNotAvailable => {
                            "Nonexistent/nonlocal address requested".to_string()
                        }
                        IoErrorKind::BrokenPipe => "Broken pipe/pipe was closed".to_string(),
                        IoErrorKind::AlreadyExists => "Entity already exists".to_string(),
                        IoErrorKind::WouldBlock => {
                            "Operation would block, but blocking not requested".to_string()
                        }
                        IoErrorKind::InvalidInput => "Parameter was incorrect".to_string(),
                        IoErrorKind::InvalidData => "Malformed input data".to_string(),
                        IoErrorKind::TimedOut => "IO timeout reached".to_string(),
                        IoErrorKind::WriteZero => "Wrote 0 bytes".to_string(),
                        IoErrorKind::Interrupted => "Operation interrupted by client".to_string(),
                        IoErrorKind::UnexpectedEof => "Premature end of file".to_string(),
                        IoErrorKind::Unsupported => {
                            "Operation unsupported on this platform".to_string()
                        }
                        IoErrorKind::OutOfMemory => "Operation ran out of memory".to_string(),
                        _ => format!(
                            "Unknown error, os code {}",
                            err.raw_os_error().unwrap_or(i32::MIN)
                        ),
                    };
                    format!(
                        "A basic input/output error happened... try again? {}",
                        err_msg1
                    )
                }
                Error::Json(err) => {
                    let line = err.line();
                    let column = err.column();

                    let err_msg1 = match err.classify() {
                        Category::Io => "Basic IO error",
                        Category::Syntax => "Invalid syntax",
                        Category::Data => "Wrong data type",
                        Category::Eof => "Premature end of file",
                    };
                    format!(
                        "Something went wrong while deserializing Discord's JSON response... try again? {} (l {} c {})",
                        err_msg1,
                        line,
                        column
                    )
                }
                Error::Model(err) => {
                    let (err_msg1, repeat) = match err {
                        ModelError::BulkDeleteAmount => {
                            ("Tried to delete too many/few messages.".to_string(), true)
                        }
                        ModelError::DeleteMessageDaysAmount(amount) => (
                            format!(
                                "Tried deleting {} days worth of messages after ban.",
                                amount
                            ),
                            true,
                        ),
                        ModelError::EmbedAmount => {
                            ("Tried sending too many embeds.".to_string(), false)
                        }
                        ModelError::EmbedTooLarge(size) => {
                            (format!("Embed content was too large ({}).", size), true)
                        }
                        ModelError::GuildNotFound => {
                            ("Guild not found in cache.".to_string(), true)
                        }
                        ModelError::RoleNotFound => ("Role not found in cache.".to_string(), false),
                        ModelError::MemberNotFound => {
                            ("Member not found in cache.".to_string(), true)
                        }
                        ModelError::ChannelNotFound => {
                            ("Channel not found in cache.".to_string(), false)
                        }
                        ModelError::MessageAlreadyCrossposted => (
                            "Tried publishing a message that was already published".to_string(),
                            false,
                        ),
                        ModelError::CannotCrosspostMessage => {
                            ("Can't publish this message.".to_string(), false)
                        }
                        ModelError::Hierarchy => (
                            "User I'm trying to act upon is higher than me in role list."
                                .to_string(),
                            false,
                        ),
                        ModelError::InvalidPermissions(perms) => {
                            let mut result =
                                String::from("I'm missing permissions to do this action: ");
                            for perm in perms.get_permission_names() {
                                result.push_str(perm)
                            }
                            (result, false)
                        }
                        ModelError::InvalidUser => {
                            ("I can't perform this action.".to_string(), false)
                        }
                        ModelError::ItemMissing => {
                            ("Item missing from cache, can't continue.".to_string(), true)
                        }
                        ModelError::WrongGuild => (
                            "Member/role/channel was provided, but for incorrect guild."
                                .to_string(),
                            false,
                        ),
                        ModelError::MessageTooLong(count) => (
                            format!(
                                "Message {} characters over 2000 unicode character limit",
                                count
                            ),
                            true,
                        ),
                        ModelError::MessagingBot => ("Can't DM another bot.".to_string(), false),
                        ModelError::InvalidChannelType => (
                            "Can't perform this action on this channel type.".to_string(),
                            false,
                        ),
                        ModelError::NameTooShort => {
                            ("Webhook name under 2 characters.".to_string(), false)
                        }
                        ModelError::NameTooLong => {
                            ("Webhook name over 100 characters.".to_string(), false)
                        }
                        ModelError::NotAuthor => {
                            ("Not the author of the message".to_string(), false)
                        }
                        ModelError::NoTokenSet => {
                            ("Don't have a webhook token set.".to_string(), false)
                        }
                        _ => ("Unknown error.".to_string(), false),
                    };

                    if repeat {
                        format!(
                            "Something went wrong while executing a action... try again? {}",
                            err_msg1
                        )
                    } else {
                        format!(
                            "Something went wrong while executing a action. {}",
                            err_msg1
                        )
                    }
                }
                Error::Num(err) => {
                    let err_msg1 = match err.kind() {
                        IntErrorKind::Empty => "was empty",
                        IntErrorKind::InvalidDigit => "was a invalid digit",
                        IntErrorKind::PosOverflow => "is too big to store in the type",
                        IntErrorKind::NegOverflow => "is too small to store in the type",
                        IntErrorKind::Zero => "was zero",
                        _ => "decided to crap out",
                    };
                    format!("Couldn't parse a integer because it {}.", err_msg1)
                }
                Error::ExceededLimit(_, _) => {
                    format!("Hit a limit when trying to do something... try again?")
                }
                Error::NotInRange(input, value, min, max) => {
                    format!(
                        "Input {} not in range. (val {}, min {}, max {})",
                        input, value, min, max
                    )
                }
                Error::Other(msg) => {
                    format!("Some other error happened. {}", msg)
                }
                Error::Url(msg) => {
                    format!("Failed to parse a URL. {}", msg)
                }
                Error::Client(err) => {
                    let err_msg1 = match err {
                        ClientError::InvalidToken => "Invalid token",
                        ClientError::ShardBootFailure => {
                            "Shard failed to restart after multiple tries"
                        }
                        ClientError::Shutdown => "All shards shutdown with error",
                        _ => "Something else went wrong",
                    };
                    format!(
                        "A client error happened. This is probably fatal. {}",
                        err_msg1
                    )
                }
                Error::Gateway(err) => {
                    let err_msg1 = match err {
                        GatewayError::BuildingUrl => "Error building URL",
                        GatewayError::Closed(_) => "Connection closed (possibly uncleanly?)",
                        GatewayError::ExpectedHello => "Expected `HELLO` during initial handshake",
                        GatewayError::HeartbeatFailed => "Error while sending `HEARTBEAT`",
                        GatewayError::InvalidAuthentication => "Bad token sent during `IDENTIFY`",
                        GatewayError::InvalidHandshake => "Expected `READY` or `INVALID_SESSION`",
                        GatewayError::InvalidOpCode => "Invalid opcode sent by gateway",
                        GatewayError::InvalidShardData => "Invalid sharding data",
                        GatewayError::NoAuthentication => "No auth sent in `IDENTIFY`",
                        GatewayError::NoSessionId => "Session ID expected but not present",
                        GatewayError::OverloadedShard => "Shard would have too many guilds on it",
                        GatewayError::ReconnectFailure => {
                            "Failed to reconnect after multiple attempts"
                        }
                        GatewayError::InvalidGatewayIntents => {
                            "Undocumented gateway intents provided"
                        }
                        GatewayError::DisallowedGatewayIntents => {
                            "Disallowed gateway intents provided"
                        }
                        _ => "Unknown error",
                    };
                    format!("A gateway error happened... try again? {}", err_msg1)
                }
                Error::Http(err) => {
                    let err_msg1 = match err.as_ref() {
                        HttpError::UnsuccessfulRequest(response) => {
                            format!("Error while making request to Discord: {:?}", response)
                        }
                        HttpError::RateLimitI64F64 => {
                            "Couldn't deserialize rate limit headers as `i64` or `f64`".to_string()
                        }
                        HttpError::RateLimitUtf8 => {
                            "Couldn't deserialize rate limit headers as valid UTF-8".to_string()
                        }
                        HttpError::Url(err) => format!("Couldn't parse URL: {}", err),
                        HttpError::InvalidHeader(h) => {
                            format!("Invalid HTTP header: {}", h)
                        }
                        HttpError::Request(r) => format!("HTTP request failure: {}", r),
                        HttpError::InvalidScheme => "Invalid proxy scheme".to_string(),
                        HttpError::InvalidPort => "Invalid proxy port".to_string(),
                        _ => "Unknown error".to_string(),
                    };
                    format!("A HTTP error happened... try again? {}", err_msg1)
                }
                Error::Rustls(err) => {
                    format!("A TLS error happened... try again? {}", err)
                }
                Error::Tungstenite(err) => {
                    format!("A WebSocket error happened... try again? {}", err)
                }
                _ => "Some other unknown error happened... try again?".to_string(),
            }
        }
    };
}
