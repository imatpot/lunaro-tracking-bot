mod commands;
mod env;
mod errors;
mod events;
mod types;
mod util;

use std::time::Duration;

use poise::serenity_prelude::{
    ActionRow, ButtonStyle, Context, CreateActionRow, CreateButton, GatewayIntents, Ready,
};
use poise::{CreateReply, Framework, FrameworkError, FrameworkOptions};
use types::error::Error;
use uuid::Uuid;

use crate::env::Environment;
use crate::events::EventHandlers;
use crate::types::error;
use crate::types::poise::PoiseContext;

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    log::debug!("Loading environment");
    let env = match Environment::load() {
        Ok(env) => env,
        Err(error) => {
            log::error!("Failed to load environment: {error}");
            return;
        }
    };

    log::debug!("Setting up");

    let framework_options = FrameworkOptions {
        pre_command: |context| Box::pin(log_invocation(context)),
        on_error: |error| Box::pin(on_error(error)),

        commands: vec![commands::ping::execute()],

        ..Default::default()
    };

    let framework = Framework::builder()
        .token(env.client_token)
        .options(framework_options)
        .intents(GatewayIntents::GUILD_MEMBERS | GatewayIntents::GUILD_PRESENCES)
        .client_settings(|client| client.raw_event_handler(EventHandlers))
        .setup(|context, ready, framework| Box::pin(update_commands(ready, context, framework)));

    log::debug!("Starting");

    match framework.run().await {
        Ok(_) => log::error!("Bot shut down unexpectedly"),
        Err(error) => log::error!("Failed to start: {error}"),
    }
}

/// Log a command's invocation.
async fn log_invocation(context: PoiseContext<'_>) {
    let author = &context.author().tag();
    let command = &context.command().name;
    let guild = &context.partial_guild().await.unwrap().name;

    log::info!("{author} ran [{command}] in {guild}");
}

/// Log an error and reply with a generic response and a button to show the
/// debug trace ID.
async fn on_error(framework_error: FrameworkError<'_, (), Error>) {
    if let FrameworkError::Command { error, ctx } = framework_error {
        let user = &ctx.author().tag();
        let command = &ctx.command().name;
        let guild = &ctx.partial_guild().await.unwrap().name;

        let trace_id = Uuid::new_v4();

        log::error!(
            "{user} ran [{command}] in {guild} and got an error {error:?}: {error} ({trace_id})",
        );

        let error_message = "❌  An error occurred while executing this command.";
        let traced_error_message = format!("{error_message}\n🔍  `{error:?}: {error} ({trace_id})`");

        let trace_button = CreateButton::default()
            .custom_id(trace_id)
            .label("Show debug trace")
            .style(ButtonStyle::Secondary)
            .to_owned();

        let action_row = CreateActionRow::default()
            .add_button(trace_button)
            .to_owned();

        let response = ctx
            .send(|reply| {
                reply
                    .ephemeral(true)
                    .content(error_message)
                    .components(|components| components.add_action_row(action_row))
            })
            .await
            .unwrap();

        let message = response.message().await.unwrap();

        match message
            .await_component_interaction(ctx)
            .timeout(Duration::from_secs(60))
            .await
        {
            Some(_) => {
                // Updates the messahe, removes the button
                response
                    .edit(ctx, |msg| {
                        msg.content(traced_error_message).components(|c| c)
                    })
                    .await
                    .unwrap();
            }
            None => {
                // Removes the button
                response
                    .edit(ctx, |msg| msg.components(|c| c))
                    .await
                    .unwrap();
            }
        };
    }
}

/// Update the bot's slash commands.
///
/// # Errors
///
/// Panics if the bot is unable to update in any guild.
async fn update_commands(
    ready: &Ready,
    context: &Context,
    framework: &Framework<(), Error>,
) -> Result<(), Error> {
    log::debug!("Updating commands");

    if let Ok(guilds) = ready.user.guilds(&context.http).await {
        for guild in guilds {
            poise::builtins::register_in_guild(
                context.http.clone(),
                &framework.options().commands,
                guild.id,
            )
            .await?;
        }
    }

    log::info!("Successfully updated commands");
    context.online().await;

    Ok(())
}
