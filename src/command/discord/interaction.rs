use itertools::Itertools;
use serenity::all::ComponentInteractionDataKind;
use serenity::builder::{
    CreateActionRow, CreateSelectMenu, CreateSelectMenuOption, EditInteractionResponse,
};
use serenity::model::application::Interaction;
use serenity::prelude::*;

use super::extensions::ClientContextExt;

pub fn menu_from_options(
    custom_id: &str,
    options: Vec<(impl ToString, impl ToString)>,
) -> Vec<CreateActionRow> {
    let menu_options = options
        .iter()
        .map(|(l, v)| CreateSelectMenuOption::new(l.to_string(), v.to_string()))
        .collect::<Vec<_>>();
    let menu = CreateSelectMenu::new(
        custom_id,
        serenity::builder::CreateSelectMenuKind::String {
            options: menu_options,
        },
    );
    vec![CreateActionRow::SelectMenu(menu)]
}

pub async fn handle_interaction(ctx: Context, interaction: Interaction) {
    match interaction {
        Interaction::Command(command) => {
            command.defer_ephemeral(&ctx.http).await.unwrap();

            let content = match command.data.name.as_str() {
                "vahti" => super::vahti::run(&ctx, &command).await,
                "poistavahti" => super::poistavahti::run(&ctx, &command).await,
                _ => unreachable!(),
            };

            if !content.is_empty() {
                command
                    .edit_response(&ctx.http, EditInteractionResponse::new().content(&content))
                    .await
                    .unwrap();
            }
        }
        Interaction::Component(button) => {
            if button.data.custom_id == "remove_vahti" {
                button.defer_ephemeral(&ctx.http).await.unwrap();
                let message = button.message.clone();
                let urls: Vec<_> = message
                    .embeds
                    .iter()
                    .filter_map(|e| e.footer.as_ref().map(|f| f.text.clone()))
                    .unique()
                    .collect();

                if !urls.is_empty() {
                    button
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new().components(menu_from_options(
                                "remove_vahti_menu",
                                urls.iter().zip(urls.iter()).collect::<Vec<_>>(),
                            )),
                        )
                        .await
                        .unwrap();
                } else {
                    button
                        .edit_response(&ctx.http,
                            EditInteractionResponse::new().content("Creating Vahti deletion menu failed, try deleting the Vahti manually with /poistavahti")
                        )
                    .await.unwrap();
                }
            } else if button.data.custom_id == "remove_vahti_menu" {
                button.defer_ephemeral(&ctx.http).await.unwrap();
                let userid = u64::from(button.user.id);
                let url = match button.data.kind.clone() {
                    ComponentInteractionDataKind::StringSelect { values } => values[0].to_string(),
                    _ => unreachable!(),
                };
                let db = ctx.get_db().await.unwrap();

                crate::vahti::remove_vahti(db, &url, userid, crate::delivery::discord::ID)
                    .await
                    .unwrap();
                button
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content(format!("Poistettu vahti: `{}`", url)),
                    )
                    .await
                    .unwrap();
            } else if button.data.custom_id.starts_with("remove_vahti_menu_page_") {
                let page_number: usize = button
                    .data
                    .custom_id
                    .strip_prefix("remove_vahti_menu_page_")
                    .unwrap()
                    .parse()
                    .unwrap();

                button
                    .create_response(
                        &ctx.http,
                        serenity::builder::CreateInteractionResponse::UpdateMessage(
                            super::poistavahti::update_message(
                                &ctx,
                                page_number,
                                u64::from(button.user.id),
                            )
                            .await,
                        ),
                    )
                    .await
                    .unwrap();
                return;
            }
        }
        _ => {}
    }
}
