use leptos::*;
use leptos_query::*;
use leptos_router::ActionForm;
use std::time::Duration;

use serde::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Todo {
    id: TodoId,
    content: String,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TodoId(u32);

#[component]
pub fn InteractiveTodo() -> impl IntoView {
    view! {
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
fn TodoWithResource() -> impl IntoView {
    let (todo_id, set_todo_id) = create_signal(TodoId(0));

    // todo_id is a Signal<String>, and that is fed into the resource fetcher function.
    // any time todo_id changes, the resource will re-execute.
    let todo_resource: Resource<TodoId, TodoResponse> = create_resource(todo_id, get_todo);

    view! {
        <div
            style:display="flex"
            style:flex-direction="column"
            style:justify-content="between"
            style:align-items="center"
            style:height="30vh"
        >
            <h2>"Todo with Resource"</h2>
            <label>"Todo ID"</label>
            <input
                type="number"
                on:input=move |ev| {
                    if let Ok(todo_id) = event_target_value(&ev).parse() {
                        set_todo_id(TodoId(todo_id));
                    }
                }

                prop:value=move || todo_id.get().0
            />
            <Transition fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                <p>
                    {move || {
                        todo_resource
                            .get()
                            .map(|a| {
                                match a.ok().flatten() {
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
fn TodoWithQuery() -> impl IntoView {
    let (todo_id, set_todo_id) = create_signal(TodoId(0));

    let QueryResult { data, .. } = todo_query().use_query(move || todo_id.get());

    view! {
        <div
            style:display="flex"
            style:flex-direction="column"
            style:justify-content="between"
            style:align-items="center"
            style:height="30vh"
        >
            <h2>"Todo with Query"</h2>
            <label>"Todo ID"</label>
            <input
                type="number"
                on:input=move |ev| {
                    if let Ok(todo_id) = event_target_value(&ev).parse() {
                        set_todo_id(TodoId(todo_id));
                    }
                }

                prop:value=move || todo_id.get().0
            />
            <Transition fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                <p>
                    {move || {
                        data.get()
                            .map(|a| {
                                match a.ok().flatten() {
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

// When using this, you get a ton of hydration errors.
#[component]
fn TodoBody(todo: Signal<Option<Option<Todo>>>) -> impl IntoView {
    view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
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
fn AllTodos() -> impl IntoView {
    let QueryResult {
        data,
        state,
        refetch,
        ..
    } = all_todos_query().use_query(|| AllTodosTag);

    let todos: Signal<Vec<Todo>> = Signal::derive(move || data.get().unwrap_or_default());

    create_effect(move |_| {
        let state = state.get();
        let log = match state {
            QueryState::Created => "created",
            QueryState::Loading => "loading",
            QueryState::Fetching(_) => "fetching",
            QueryState::Loaded(_) => "loaded",
            QueryState::Invalid(_) => "invalid",
        };
        logging::log!("STATE: {log}")
    });

    let delete_todo = create_action(move |id: &TodoId| {
        let id = *id;
        let refetch = refetch.clone();

        let todo_query = todo_query();
        let todos = all_todos_query();
        async move {
            todos.cancel_query(AllTodosTag);

            todos.update_query_data_mut(AllTodosTag, |todos| {
                todos.retain(|t| t.id != id);
            });

            todo_query.set_query_data(id, Ok(None));

            // Delete todos on the server.
            let _ = delete_todo(id).await;

            let _ = todo_query.invalidate_query(id);

            refetch()
        }
    });

    view! {
        <h2>"All Todos"</h2>
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            <ul>
                <Show
                    when=move || !todos.get().is_empty()
                    fallback=|| {
                        view! { <p>"No todos"</p> }
                    }
                >

                    <For
                        each=todos
                        key=|todo| todo.id
                        children=move |todo| {
                            view! {
                                <li>
                                    <span>{todo.id.0}</span>
                                    <span>": "</span>
                                    <span>{todo.content}</span>
                                    <span>" "</span>
                                    <button on:click=move |_| {
                                        delete_todo.dispatch(todo.id)
                                    }>"X"</button>
                                </li>
                            }
                        }
                    />

                </Show>
            </ul>
        </Transition>
    }
}

#[component]
fn AddTodoComponent() -> impl IntoView {
    let add_todo = create_server_action::<AddTodo>();

    let response = add_todo.value();

    let todo_query = todo_query();
    let all_todos = all_todos_query();

    create_effect(move |_| {
        // If action is successful.
        if let Some(Ok(todo)) = response.get() {
            all_todos.cancel_query(AllTodosTag);

            // Optimistic update for all todos.
            all_todos.update_query_data_mut(AllTodosTag, {
                let todo = todo.clone();
                |todos| {
                    todos.push(todo);
                }
            });

            // Optimistic update for individual TodoResponse.
            let id = todo.id.clone();
            todo_query.set_query_data(id.clone(), Ok(Some(todo)));

            // Invalidate individual TodoResponse.
            todo_query.invalidate_query(id);

            // Invalidate AllTodos.
            all_todos.invalidate_query(AllTodosTag);
        }
    });

    view! {
        <ActionForm action=add_todo>
            <label>"Add a Todo " <input type="text" name="content"/></label>
            <input type="submit" autocomplete="off" value="Add"/>
        </ActionForm>
    }
}

/**
 * Todo Helpers.
 */
fn todo_query() -> QueryScope<TodoId, TodoResponse> {
    create_query(
        get_todo,
        QueryOptions {
            stale_time: Some(Duration::from_secs(5)),
            ..Default::default()
        },
    )
}

/**
 * All Todos Helpers.
 */

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct AllTodosTag;

fn all_todos_query() -> QueryScope<AllTodosTag, Vec<Todo>> {
    create_query(
        |_| async move { get_todos().await.unwrap_or_default() },
        QueryOptions {
            stale_time: Some(Duration::from_secs(5)),
            ..Default::default()
        },
    )
}

cfg_if::cfg_if! {
    if #[cfg(feature = "ssr")] {
        use std::{sync::RwLock};
        static GLOBAL_TODOS: RwLock<Vec<Todo>> = RwLock::new(vec![]);
    }
}

// Read.

type TodoResponse = Result<Option<Todo>, ServerFnError>;

#[server(GetTodo, "/api")]
async fn get_todo(id: TodoId) -> Result<Option<Todo>, ServerFnError> {
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

// Mutate.

#[server(AddTodo, "/api")]
pub async fn add_todo(content: String) -> Result<Todo, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let mut todos = GLOBAL_TODOS.write().unwrap();

    let new_id = todos
        .last()
        .map(|t| t.id.0 + 1)
        .map(TodoId)
        .unwrap_or(TodoId(0));

    let new_todo = Todo {
        id: new_id,
        content,
    };

    todos.push(new_todo.clone());

    Ok(new_todo)
}

#[server(DeleteTodo, "/api")]
async fn delete_todo(id: TodoId) -> Result<(), ServerFnError> {
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let mut todos = GLOBAL_TODOS.write().unwrap();
    todos.retain(|t| t.id != id);
    Ok(())
}
