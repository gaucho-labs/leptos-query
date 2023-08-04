use leptos::*;
use leptos_query::*;
use leptos_router::ActionForm;
use std::{sync::RwLock, time::Duration};

use serde::*;
#[derive(Serialize, Deserialize, Clone)]
pub struct Todo {
    id: u32,
    content: String,
}

#[component]
pub fn InteractiveTodo(cx: Scope) -> impl IntoView {
    view! { cx,
        <div>
            <div style:display="flex" style:gap="10rem">
                <TodoWithResource/>
                <TodoWithQuery/>
            </div>
            <AddTodoComponent/>
            <AllTodos/>
        </div>
    }
}

#[component]
fn TodoWithResource(cx: Scope) -> impl IntoView {
    let (todo_id, set_todo_id) = create_signal(cx, 0_u32);

    // todo_id is a Signal<String>, and that is fed into the resource fetcher function.
    // any time todo_id changes, the resource will re-execute.
    let todo_resource: Resource<u32, Option<Todo>> =
        create_resource(cx, todo_id, |id| async move { get_todo(id).await.unwrap() });

    let todo = Signal::derive(cx, move || todo_resource.read(cx));

    view! { cx,
        <div
            style:display="flex"
            style:flex-direction="column"
            style:justify-content="between"
            style:align-items="center"
            style:height="30vh"
        >
            <h2>"Todo with Resource" </h2>
            <label>"Todo ID"</label>
            <input
                type="number"
                on:input=move |ev| {
                    if let Ok(todo_id) = event_target_value(&ev).parse() {
                        set_todo_id(todo_id);
                    }
                }
                prop:value=todo_id
            />
            <Transition fallback=move || {
                view! { cx, <p>"Loading..."</p> }
            }>
                <p>
                    {move || {
                        todo.get()
                            .map(|a| {
                                match a {
                                    Some(todo) => todo.content,
                                    None => "Not found".into(),
                                }
                            })
                    }}
                </p>
            </Transition>
        </div>
    }
}

#[component]
fn TodoWithQuery(cx: Scope) -> impl IntoView {
    let (todo_id, set_todo_id) = create_signal(cx, 0_u32);

    let QueryResult { data, .. } = use_query(
        cx,
        todo_id,
        |id| async move { get_todo(id).await.unwrap() },
        QueryOptions::default(),
    );

    view! { cx,
        <div
            style:display="flex"
            style:flex-direction="column"
            style:justify-content="between"
            style:align-items="center"
            style:height="30vh"
        >
            <h2>"Todo with Query" </h2>
            <label>"Todo ID"</label>
            <input
                type="number"
                on:input=move |ev| {
                    if let Ok(todo_id) = event_target_value(&ev).parse() {
                        set_todo_id(todo_id);
                    }
                }
                prop:value=todo_id
            />
            <Transition fallback=move || {
                view! { cx, <p>"Loading..."</p> }
            }>
                <p>
                    {move || {
                        data.get()
                            .map(|a| {
                                match a {
                                    Some(todo) => todo.content,
                                    // This case breaks the hydration on SSR.
                                    None => "Not found".into(),
                                }
                            })
                    }}
                </p>
            </Transition>
        </div>
    }
}

// When using this, you get a ton of hydration errors.
#[component]
fn TodoBody(cx: Scope, todo: Signal<Option<Option<Todo>>>) -> impl IntoView {
    view! { cx,
        <Suspense fallback=move || {
            view! { cx, <p>"Loading..."</p> }
        }>
            <p>
                {move || {
                    todo.get()
                        .map(|a| {
                            match a {
                                Some(todo) => todo.content,
                                None => "Not found".into(),
                            }
                        })
                }}
            </p>
        </Suspense>
    }
}

#[component]
fn AllTodos(cx: Scope) -> impl IntoView {
    let QueryResult { data, refetch, .. } = use_query(
        cx,
        || (),
        |_| async move { get_todos().await.unwrap_or_default() },
        QueryOptions::default(),
    );

    let todos: Signal<Vec<Todo>> = Signal::derive(cx, move || data.get().unwrap_or_default());

    let delete_todo = create_action(cx, move |id: &u32| {
        let id = *id;
        let refetch = refetch.clone();
        async move {
            let _ = delete_todo(id.clone()).await;
            refetch();
            use_query_client(cx).invalidate_query::<u32, Option<Todo>>(&id);
        }
    });

    view! { cx,
        <h2>"All Todos"</h2>
        <Suspense fallback=move || {
            view! { cx, <p>"Loading..."</p> }
        }>
            <ul>
                <For
                    each=todos
                    key=|todo| todo.id
                    view=move |cx, todo| {
                        view! { cx,
                            <li>
                                <span>{todo.id}</span>
                                <span>": "</span>
                                <span>{todo.content}</span>
                                <span>" "</span>
                                <button on:click=move |_| delete_todo.dispatch(todo.id) >
                                    "X"
                                </button>
                            </li>
                        }
                    }
                />
            </ul>
        </Suspense>
    }
}

#[component]
fn AddTodoComponent(cx: Scope) -> impl IntoView {
    let add_todo = create_server_action::<AddTodo>(cx);

    let response = add_todo.value();

    let client = use_query_client(cx);

    create_effect(cx, move |_| {
        if response.get().is_some() {
            client.clone().invalidate_all_queries::<u32, Option<Todo>>();
            client.clone().invalidate_all_queries::<(), Vec<Todo>>();
        }
    });

    view! { cx,
        <ActionForm action=add_todo>
            <label>"Add a Todo " <input type="text" name="content"/></label>
            <input type="submit" value="Add"/>
        </ActionForm>
    }
}

#[cfg(feature = "ssr")]
static GLOBAL_TODOS: RwLock<Vec<Todo>> = RwLock::new(vec![]);

#[server(GetTodo, "/api")]
async fn get_todo(id: u32) -> Result<Option<Todo>, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let todos = GLOBAL_TODOS.read().unwrap();
    Ok(todos.iter().find(|t| t.id == id).cloned())
}

#[server(GetTodos, "/api")]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let todos = GLOBAL_TODOS.read().unwrap();
    Ok(todos.clone())
}

#[server(DeleteTodo, "/api")]
async fn delete_todo(id: u32) -> Result<(), ServerFnError> {
    let mut todos = GLOBAL_TODOS.write().unwrap();
    todos.retain(|t| t.id != id);
    Ok(())
}

#[server(AddTodo, "/api")]
pub async fn add_todo(content: String) -> Result<(), ServerFnError> {
    let mut todos = GLOBAL_TODOS.write().unwrap();

    let new_id = todos.last().map(|t| t.id + 1).unwrap_or(0);

    todos.push(Todo {
        id: new_id as u32,
        content,
    });

    Ok(())
}
