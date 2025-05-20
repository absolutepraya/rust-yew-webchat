use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};
use chrono::{DateTime, Utc, TimeZone, NaiveDateTime};
use std::collections::HashMap;

use crate::{User, services::websocket::WebsocketService};
use crate::services::event_bus::EventBus;

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
    ReplyTo(usize),
    CancelReply,
}

#[derive(Deserialize, Clone)]
struct MessageData {
    from: String,
    message: String,
    time: Option<i64>,
    reply_to: Option<ReplyData>,
}

#[derive(Deserialize, Serialize, Clone)]
struct ReplyData {
    id: usize,
    from: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Clone)]
struct UserProfile {
    name: String,
    avatar: String,
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    wss: WebsocketService,
    messages: Vec<MessageData>,
    _producer: Box<dyn Bridge<EventBus>>,
    replying_to: Option<(usize, MessageData)>,
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");
        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        let message = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.to_string()),
            data_array: None,
        };

        if let Ok(_) = wss
            .tx
            .clone()
            .try_send(serde_json::to_string(&message).unwrap())
        {
            log::debug!("message sent successfully");
        }

        Self {
            users: vec![],
            messages: vec![],
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
            replying_to: None,
        }
    }
    
    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(s) => {
                let msg: WebSocketMessage = serde_json::from_str(&s).unwrap();
                match msg.message_type {
                    MsgTypes::Users => {
                        let users_from_message = msg.data_array.unwrap_or_default();
                        self.users = users_from_message
                            .iter()
                            .map(|u| UserProfile {
                                name: u.into(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                )
                                .into(),
                            })
                            .collect();
                        return true;
                    }
                    MsgTypes::Message => {
                        let message_data: MessageData =
                            serde_json::from_str(&msg.data.unwrap()).unwrap();
                        self.messages.push(message_data);
                        return true;
                    }
                    _ => {
                        return false;
                    }
                }
            }
            Msg::SubmitMessage => {
                let input = self.chat_input.cast::<HtmlInputElement>();
                if let Some(input) = input {
                    if !input.value().trim().is_empty() {
                        let mut data_to_send = HashMap::new();
                        data_to_send.insert("text", input.value());
                        
                        // Add reply data if we're replying to a message
                        if let Some((id, ref msg)) = self.replying_to {
                            let reply_data = ReplyData {
                                id,
                                from: msg.from.clone(),
                                message: msg.message.clone(),
                            };
                            data_to_send.insert("reply_to", serde_json::to_string(&reply_data).unwrap());
                        }
                        
                        let message = WebSocketMessage {
                            message_type: MsgTypes::Message,
                            data: Some(serde_json::to_string(&data_to_send).unwrap()),
                            data_array: None,
                        };
                        
                        if let Err(e) = self
                            .wss
                            .tx
                            .clone()
                            .try_send(serde_json::to_string(&message).unwrap())
                        {
                            log::debug!("error sending to channel: {:?}", e);
                        }
                        
                        input.set_value("");
                        self.replying_to = None;
                        return true;
                    }
                };
                false
            }
            Msg::ReplyTo(index) => {
                if index < self.messages.len() {
                    self.replying_to = Some((index, self.messages[index].clone()));
                    return true;
                }
                false
            }
            Msg::CancelReply => {
                if self.replying_to.is_some() {
                    self.replying_to = None;
                    return true;
                }
                false
            }
        }
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);
        let cancel_reply = ctx.link().callback(|_| Msg::CancelReply);
        
        html! {
            <div class="flex w-screen">
                <div class="flex-none w-56 h-screen bg-gray-100">
                    <div class="text-xl p-3">{"Users"}</div>
                    {
                        self.users.clone().iter().map(|u| {
                            html!{
                                <div class="flex m-3 bg-white rounded-lg p-2">
                                    <div>
                                        <img class="w-12 h-12 rounded-full" src={"https://res.cloudinary.com/dr1tp0gwd/image/upload/v1747738474/mnzlvv15ooei5t3xusua.png"} alt="avatar"/>
                                    </div>
                                    <div class="flex-grow p-3">
                                        <div class="flex text-xs justify-between">
                                            <div>{u.name.clone()}</div>
                                        </div>
                                        <div class="text-xs text-gray-400">
                                            {"Hi there!"}
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Html>()
                    }
                </div>
                <div class="grow h-screen flex flex-col">
                    <div class="w-full h-14 border-b-2 border-gray-300"><div class="text-xl p-3">{"💬 Chat!"}</div></div>
                    <div class="w-full grow overflow-auto border-b-2 border-gray-300">
                        {
                            self.messages.iter().enumerate().map(|(index, m)| {
                                let user = self.users.iter().find(|u| u.name == m.from).unwrap();
                                let timestamp = match m.time {
                                    Some(t) => {
                                        if let Some(dt) = NaiveDateTime::from_timestamp_millis(t) {
                                            let datetime: DateTime<Utc> = Utc.from_utc_datetime(&dt);
                                            format!("{}", datetime.format("%H:%M:%S"))
                                        } else {
                                            "".to_string()
                                        }
                                    },
                                    None => "".to_string(),
                                };
                                
                                let reply_callback = ctx.link().callback(move |_| Msg::ReplyTo(index));
                                
                                html!{
                                    <div class="flex flex-col items-end w-3/6 bg-gray-100 m-8 rounded-tl-lg rounded-tr-lg rounded-br-lg ">
                                        {
                                            if let Some(ref reply) = m.reply_to {
                                                html! {
                                                    <div class="bg-gray-200 w-11/12 mx-auto mt-2 p-2 rounded-lg border-l-4 border-blue-500">
                                                        <div class="text-xs font-semibold text-blue-600">
                                                            {format!("↩️ Reply to {}", reply.from)}
                                                        </div>
                                                        <div class="text-xs text-gray-500 truncate">
                                                            {reply.message.clone()}
                                                        </div>
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }
                                        }
                                        <div class="flex items-end w-full">
                                            <img class="w-8 h-8 rounded-full m-3" src={"https://res.cloudinary.com/dr1tp0gwd/image/upload/v1747738474/mnzlvv15ooei5t3xusua.png"} alt="avatar"/>
                                            <div class="p-3 w-full">
                                                <div class="text-sm flex justify-between">
                                                    <span>{m.from.clone()}</span>
                                                    <span class="text-xs text-gray-400">{timestamp}</span>
                                                </div>
                                                <div class="text-xs text-gray-500">
                                                    if m.message.ends_with(".gif") {
                                                        <img class="mt-3" src={m.message.clone()}/>
                                                    } else {
                                                        {m.message.clone()}
                                                    }
                                                </div>
                                            </div>
                                            <button onclick={reply_callback} class="p-2 m-2 text-blue-500 hover:bg-blue-100 rounded-full">
                                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                    <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"></path>
                                                </svg>
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }

                    </div>
                    <div class="w-full flex flex-col">
                        {
                            if let Some((_, ref msg)) = self.replying_to {
                                html! {
                                    <div class="flex items-center bg-blue-50 px-4 py-2">
                                        <div class="flex-grow">
                                            <div class="text-xs text-blue-600 font-semibold">
                                                {format!("Replying to {}", msg.from)}
                                            </div>
                                            <div class="text-xs text-gray-500 truncate">
                                                {msg.message.clone()}
                                            </div>
                                        </div>
                                        <button onclick={cancel_reply} class="text-gray-500 hover:text-gray-700">
                                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                                <line x1="6" y1="6" x2="18" y2="18"></line>
                                            </svg>
                                        </button>
                                    </div>
                                }
                            } else {
                                html! {}
                            }
                        }
                        <div class="w-full h-14 flex px-3 items-center">
                            <input ref={self.chat_input.clone()} type="text" placeholder="Message" class="block w-full py-2 pl-4 mx-3 bg-gray-100 rounded-full outline-none focus:text-gray-700" name="message" required=true />
                            <button onclick={submit} class="p-3 shadow-sm bg-blue-600 w-10 h-10 rounded-full flex justify-center items-center color-white">
                                <svg fill="#000000" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="fill-white">
                                    <path d="M0 0h24v24H0z" fill="none"></path><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        }
    }
}