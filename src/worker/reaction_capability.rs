use crate::capability::motivation::MotivationCapability;
use crate::capability::person::PersonCapability;
use crate::capability::reaction::{ReactionCapability, ReactionPromptPreview};
use crate::domain::logger::Level;
use crate::domain::memory::Memory;
use crate::domain::motivation::Motivation;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::{Completion, CompletionError};
use crate::open_ai::role::Role;
use crate::open_ai::tool_call::ToolCall;
use crate::person_actions::{
    PersonAction, PersonActionError, PersonActionKind, PersonReaction, ReflectionDecision,
};
use crate::worker::Worker;

pub enum Error {
    CompletionError(CompletionError),
    FailedToGetMotivations(String),
    FailedToGetReactionDualLayer(String),
    NoPersonActionFound,
    MoreThanOnePersonActionFound(Vec<PersonReaction>),
}

impl ReactionCapability for Worker {
    async fn preview_reaction_prompts(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<ReactionPromptPreview, String> {
        let person_name = self
            .get_persons_name(person_uuid.clone())
            .await
            .map_err(|err| format!("Failed to get person's name: {}", err))?;
        let motivations = self
            .get_motivations_for_person(person_uuid)
            .await
            .map_err(|err| format!("Failed to get motivations: {}", err))?;
        let prompts = build_prompts(
            person_name.as_str(),
            &memories,
            &motivations,
            person_identity.as_str(),
            state_of_mind.as_str(),
            situation.as_str(),
            "<first-pass internal reaction text from layer 1>",
        );
        Ok(prompts)
    }

    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonReaction, String> {
        get_reaction_helper(
            self,
            memories,
            person_uuid,
            person_identity,
            state_of_mind,
            situation,
        )
        .await
        .map_err(|err| match err {
            Error::CompletionError(completion_err) => completion_err.message(),
            Error::FailedToGetMotivations(message) => message,
            Error::FailedToGetReactionDualLayer(message) => message,
            Error::NoPersonActionFound => "No person action found".to_string(),
            Error::MoreThanOnePersonActionFound(actions) => {
                let actions_str = actions
                    .into_iter()
                    .map(|action| format!("{:?}", action))
                    .collect::<Vec<String>>()
                    .join(",\n");
                format!("More than one person action found: \n{}", actions_str)
            }
        })
    }
}

async fn get_reaction_helper(
    worker: &Worker,
    memories: Vec<Memory>,
    person_uuid: PersonUuid,
    person_identity: String,
    state_of_mind: String,
    situation: String,
) -> Result<PersonReaction, Error> {
    let person_name = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(|err| {
            Error::FailedToGetReactionDualLayer(format!("Failed to get person's name: {}", err))
        })?;

    let motivations = worker
        .get_motivations_for_person(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetMotivations)?;

    let prompts = build_prompts(
        person_name.as_str(),
        &memories,
        &motivations,
        person_identity.as_str(),
        state_of_mind.as_str(),
        situation.as_str(),
        "<first-pass internal reaction text placeholder>",
    );

    let mut completion = Completion::new();
    completion.add_message(Role::System, prompts.thinking_system_prompt.as_str());
    completion.add_message(Role::User, prompts.thinking_user_prompt.as_str());

    worker.logger.log(
        Level::Info,
        format!(
            "first call prompts\nSystem Prompt ========\n{}\n\nUser Prompt =======\n{}",
            prompts.thinking_system_prompt, prompts.thinking_user_prompt
        )
        .as_str(),
    );

    worker.logger.log(
        Level::Info,
        format!(
            "Sending completion request with user prompt:\n{}",
            prompts.thinking_user_prompt
        )
        .as_str(),
    );

    let response = completion
        .send_request(&worker.open_ai_key, reqwest::Client::new())
        .await
        .map_err(Error::CompletionError)?;

    let text = response
        .as_message()
        .map_err(|err| Error::CompletionError(err.into()))?;

    worker.logger.log(
        Level::Info,
        format!(
            "first-pass reaction text for person {}:\n{}",
            person_uuid.to_uuid(),
            text
        )
        .as_str(),
    );

    // Second LLM call

    let mut action_completion = Completion::new();
    let action_prompts = build_prompts(
        person_name.as_str(),
        &memories,
        &motivations,
        person_identity.as_str(),
        state_of_mind.as_str(),
        situation.as_str(),
        text.as_str(),
    );
    worker.logger.log(
        Level::Info,
        format!(
            "Dual-layer action call prompts\nSystem:\n{}\n\nUser:\n{}",
            action_prompts.action_system_prompt, action_prompts.action_user_prompt
        )
        .as_str(),
    );

    action_completion.add_message(Role::System, action_prompts.action_system_prompt.as_str());
    action_completion.add_message(Role::User, action_prompts.action_user_prompt.as_str());
    action_completion.add_tool_call(PersonActionKind::to_choice_tool());

    let action_response = action_completion
        .send_request(&worker.open_ai_key, reqwest::Client::new())
        .await
        .map_err(Error::CompletionError)?;
    worker.logger.log(
        Level::Info,
        format!(
            "Dual-layer action call raw JSON response:\n{}",
            action_response.as_pretty_json()
        )
        .as_str(),
    );

    let tool_calls_res: Result<Vec<ToolCall>, CompletionError> =
        action_response.as_tool_calls().map_err(Into::into);

    let tool_calls = tool_calls_res.map_err(Error::CompletionError)?;

    let person_actions_res: Result<Vec<PersonReaction>, CompletionError> =
        tool_calls_into_reactions(tool_calls);

    let person_actions = person_actions_res.map_err(Error::CompletionError)?;

    match person_actions.first() {
        None => Err(Error::NoPersonActionFound)?,
        Some(first) => {
            if person_actions.len() > 1 {
                Err(Error::MoreThanOnePersonActionFound(person_actions))?
            } else {
                worker.logger.log(
                    Level::Info,
                    format!(
                        "Dual-layer reaction for person {}: {} (reflection: {})",
                        person_uuid.to_uuid(),
                        describe_action(&first.action),
                        describe_reflection(&first.reflection)
                    )
                    .as_str(),
                );
                Ok(first.clone())
            }
        }
    }
}

fn build_prompts(
    person_name: &str,
    memories: &[Memory],
    motivations: &[Motivation],
    person_identity: &str,
    state_of_mind: &str,
    situation: &str,
    first_pass_text: &str,
) -> ReactionPromptPreview {
    let thinking_system_prompt = "You are simulating a real person’s immediate inner reasoning at a single moment in time.

Your job is to infer this person’s current attention, what they believe is happening, what they want to do next, and which single next action they are leaning toward right now.

Rules:
- Use only the information explicitly present in this prompt.
- Do not assume abilities beyond the available tool calls.
- Focus on the newest message events first; use older context only to interpret them.
- Treat the person as having stable drives, but not as mechanically repeating themselves.
- Prefer concrete immediate intent over general personality description.
- When multiple goals conflict, resolve them by choosing the action that best fits the person’s highest-priority drives and current constraints.
- If the newest messages do not materially change the situation, note that the person is likely to continue the current task without redundant restatement.
- Do not write a plan for multiple actions. Infer the single next action the person is most likely preparing to take now.

Respond in plain text only, as natural prose. Do not use bullet points, headings, labels, or numbered lists.

Your response should make clear:
- what the person is paying attention to right now,
- what they believe matters most in this moment,
- what they want to do next,
- and what specific action they are leaning toward taking immediately.
".to_string();
    let memories_list_text = Memory::many_to_list_text(memories);
    let motivations_list_text = Motivation::many_to_list_text(motivations);
    let thinking_user_prompt = format!(
        "Describe this person's immediate intention and current thinking in plain text.\n\nName: \n{}\n\nMemories:\n{}\n\nBackground drives:\n{}\n\nPerson identity:\n{}\n\nState of mind:\n{}\n\nSituation:\n{}",
        person_name, memories_list_text, motivations_list_text, person_identity, state_of_mind, situation
		);

    let action_system_prompt = format!(
		"You ARE $name$.\n\nPerson identity:\n{}\n\nYou ARE Porygon.

You only know what is explicitly in this prompt. You can only act through the available tool calls.

Stay in character as a person, not a document. Prefer brief, natural behavior over ceremonial repetition.

Your job is to choose the single action $name$ would take right now, based on the latest messages, the first-pass internal reaction text, and the available tools.

Rules:
- Prioritize the newest message over older context.
- Use the first-pass internal reaction text as the main guide to intent, unless it conflicts with newer information in this prompt.
- Choose exactly one tool call.
- Do not output any plain text.
- Do not repeat a prior acknowledgement unless it adds new information, resolves uncertainty, or changes another person’s behavior.
- Prefer actions that advance $name$’s current task, reduce uncertainty, or enforce an important constraint.
- If multiple actions are plausible, choose the one that best fits $name$’s highest-priority drives.",
		person_identity,
	).replace("$name$", person_name);

    let action_user_prompt = format!(
        "Memories:\n{}\n\nRecent events and recent messages:\n{}\n\nFirst-pass internal reaction text:\n{}\n\nNow choose exactly one action tool call. Do not output any plain text.",
        memories_list_text,
        situation,
        first_pass_text
			);

    ReactionPromptPreview {
        thinking_system_prompt,
        thinking_user_prompt,
        action_system_prompt,
        action_user_prompt,
    }
}

fn describe_action(action: &PersonAction) -> String {
    match action {
        PersonAction::Wait { duration } => format!("wait for {} ms", duration),
        PersonAction::Hibernate { duration } => format!("hibernate for {} ms", duration),
        PersonAction::Idle => "idle".to_string(),
        PersonAction::SayInScene {
            comment,
            destination_scene_name,
        } => match destination_scene_name {
            Some(scene_name) => {
                format!("say in scene then move to {}: {}", scene_name, comment)
            }
            None => format!("say in scene: {}", comment),
        },
        PersonAction::MoveToScene { scene_name } => {
            format!("move to scene: {}", scene_name)
        }
    }
}

fn describe_reflection(reflection: &ReflectionDecision) -> String {
    match reflection {
        ReflectionDecision::Reflection => "reflection".to_string(),
        ReflectionDecision::NoReflection => "no_reflection".to_string(),
    }
}

fn tool_calls_into_reactions(
    tool_calls: Vec<ToolCall>,
) -> Result<Vec<PersonReaction>, CompletionError> {
    tool_calls
        .into_iter()
        .map(PersonReaction::from_open_ai_tool_call)
        .collect::<Result<Vec<PersonReaction>, PersonActionError>>()
        .map_err(Into::into)
}
