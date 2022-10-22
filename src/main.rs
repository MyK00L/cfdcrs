use poise::serenity_prelude as serenity;
use std::time::{Duration, SystemTime};

mod tokens;

type Error = Box<dyn std::error::Error + Send + Sync>;
// User data, which is stored and accessible in all command invocations
struct Data {
    token_db: tokens::DbType,
}
type Context<'a> = poise::Context<'a, Data, Error>;

/// Displays a user's roles
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
async fn roles(ctx: Context<'_>, member: serenity::Member) -> Result<(), Error> {
    let response = format!("{:?}", member.roles(ctx.discord()));
    ctx.say(response).await?;
    Ok(())
}

/// Gives a role to a member
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
async fn give_role(
    ctx: Context<'_>,
    mut member: serenity::Member,
    role: serenity::Role,
) -> Result<(), Error> {
    member.add_role(ctx.discord(), role).await?;
    Ok(())
}

/// Removes a role from a member
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
async fn remove_role(
    ctx: Context<'_>,
    mut member: serenity::Member,
    role: serenity::Role,
) -> Result<(), Error> {
    member.remove_role(ctx.discord(), role).await?;
    Ok(())
}

/// Adds a token for a role
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
async fn add_token(
    ctx: Context<'_>,
    role0: serenity::Role,
    role1: Option<serenity::Role>,
    role2: Option<serenity::Role>,
    role3: Option<serenity::Role>,
    uses: Option<u32>,
    time_limit_hours: Option<u64>,
) -> Result<(), Error> {
    let roles: Vec<serenity::RoleId> = [Some(role0), role1, role2, role3]
        .into_iter()
        .flatten()
        .map(|x| x.id)
        .collect();
    let key = tokens::add_token(
        ctx.data().token_db.clone(),
        tokens::TokenData {
            roles,
            limit: uses.unwrap_or(1),
            expiration: SystemTime::now()
                + Duration::from_secs(3600 * time_limit_hours.unwrap_or(24 * 4)),
        },
    )
    .await?;
    ctx.say(format!("{}", key)).await?;
    Ok(())
}

/// Removes a token
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
async fn remove_token(ctx: Context<'_>, token: u128) -> Result<(), Error> {
    tokens::rem_token(ctx.data().token_db.clone(), token).await?;
    ctx.say("success").await?;
    Ok(())
}

/// Use a token to gain a role
#[poise::command(slash_command, guild_only)]
async fn use_token(ctx: Context<'_>, token: u128) -> Result<(), Error> {
    let old_roles: Vec<serenity::RoleId> = ctx
        .author_member()
        .await
        .ok_or("cannot fetch message sender as a member")?
        .to_mut()
        .roles(ctx.discord())
        .unwrap_or_default()
        .iter()
        .map(|x| x.id)
        .collect();
    let mut roles: Vec<serenity::RoleId> =
        tokens::use_token(ctx.data().token_db.clone(), token).await?;
    roles.sort();
    roles.dedup();
    roles.retain(|x| !old_roles.contains(x));
    ctx.author_member()
        .await
        .ok_or("cannot fetch message sender as a member")?
        .to_mut()
        .add_roles(ctx.discord(), &roles)
        .await?;
    ctx.say("success").await?;
    Ok(())
}

/// Registers or unregisters application commands in this guild or globally
#[poise::command(prefix_command, hide_in_help, required_permissions = "ADMINISTRATOR")]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("env");
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                register(),
                roles(),
                give_role(),
                remove_role(),
                add_token(),
                remove_token(),
                use_token(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("~".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .user_data_setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    token_db: tokens::load_db().await.unwrap(),
                })
            })
        });

    framework.run().await.unwrap();
}
