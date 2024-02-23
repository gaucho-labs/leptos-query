use std::{cell::RefCell, time::Duration};

use leptos::*;
use leptos_query::{create_query, QueryOptions, QueryScope};
use serde::{Deserialize, Serialize};

use crate::components::{header::Header, skeleton::Skeleton, spinner::Spinner};

#[component]
pub fn Interactive() -> impl IntoView {
    view! {
        <div class="mx-auto max-w-xl flex flex-col w-full h-full items-center gap-10">
            <Header title="Optimistic Update">
                <p>"Each todo operation takes 1 second, but the UI feels instant."</p>
            </Header>

            <AddTodoEntry/>
            <AllTodos/>
        </div>
    }
}

#[component]
pub fn AllTodos() -> impl IntoView {
    let query = all_todos_query().use_query(|| AllTodosKey);
    let todos = query.data;

    view! {
        <Transition fallback=move || {
            view! {
                <div class=CARD_CLASS>
                    <Skeleton class="h-8 w-full"/>
                    <Skeleton class="h-20 w-full"/>
                </div>
            }
        }>
            {move || {
                todos
                    .get()
                    .map(|todos| {
                        view! {
                            <ul class="flex flex-col w-full gap-2">
                                <For
                                    each=move || todos.clone()
                                    key=|todo| todo.id
                                    children=move |todo| {
                                        view! { <TodoListItem todo=todo/> }
                                    }
                                />

                            </ul>
                        }
                    })
            }}

        </Transition>
    }
}

const CARD_CLASS: &str = "flex flex-col items-start justify-between w-full rounded-xl border bg-card text-card-foreground shadow-md p-6 gap-2";

#[component]
fn TodoListItem(todo: Todo) -> impl IntoView {
    let delete = move |id: TodoId| async move {
        let all_todos = all_todos_query();

        all_todos.cancel_query(AllTodosKey);

        all_todos.update_query_data_mut(AllTodosKey, move |todos| {
            todos.retain(|t| t.id != id);
        });

        let _ = delete_todo(id).await;

        all_todos.invalidate_query(AllTodosKey);
    };

    view! {
        <li class=CARD_CLASS>
            <div class="flex items-center justify-between w-full">
                <div class="text-base">{todo.title}</div>
                // <div class="line-clamp-1 text-muted-foreground text-sm">{todo.content}</div>
                <button
                    class="inline-flex items-center justify-center whitespace-nowrap text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-9 rounded-md px-3"
                    on:click=move |_| {
                        spawn_local(delete(todo.id));
                    }
                >

                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        class="w-4 h-4"
                        data-id="19"
                    >
                        <path d="M3 6h18"></path>
                        <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path>
                        <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path>
                    </svg>
                    <span class="sr-only" data-id="20">
                        Delete
                    </span>
                </button>
            </div>
            <div class="text-muted-foreground text-sm text-wrap w-full overflow-hidden">
                {todo.content}
            </div>
        </li>
    }
}

#[component]
fn AddTodoEntry() -> impl IntoView {
    let form_ref = create_node_ref::<html::Form>();

    let titleX = create_rw_signal("".to_string());
    let contentX = create_rw_signal("".to_string());
    let loading = create_rw_signal(false);

    let add_todo = move || {
        let all_todos = all_todos_query();
        spawn_local(async move {
            all_todos.cancel_query(AllTodosKey);

            let title = titleX.get_untracked();
            let content = contentX.get_untracked();

            titleX.set(Default::default());
            contentX.set(Default::default());

            // Find a unique id for the todo.
            let temp_id = {
                let temp_id = all_todos
                    .peek_query_state(&AllTodosKey)
                    .and_then(|todos| {
                        let todos = todos.data()?;
                        let id = todos.iter().map(|t| t.id.0).max()?;
                        Some(id + 1)
                    })
                    .unwrap_or(0) as u32;

                TodoId(temp_id)
            };

            // Optimistically add the todo to the list
            all_todos.update_query_data_mut(AllTodosKey, {
                let title = title.clone();
                let content = content.clone();
                |todos| {
                    todos.push(Todo {
                        id: temp_id,
                        title,
                        content,
                        completed: false,
                    })
                }
            });

            loading.set(true);
            let todo = add_todo(title, content).await;
            loading.set(false);

            // Replace the optimistic todo with the real todo
            all_todos.update_query_data_mut(AllTodosKey, {
                move |todos| {
                    todos.retain(|t| t.id != temp_id);
                    todos.push(todo);
                }
            });

            all_todos.invalidate_query(AllTodosKey);
        })
    };

    view! {
        <form
            node_ref=form_ref
            class="flex flex-col items-start gap-2 w-full bg-card 100 p-4 rounded-xl shadow border"
            on:submit=move |event| {
                event.prevent_default();
                add_todo()
            }
        >

            <label for="title">Title</label>
            <input
                type="text"
                autocomplete="off"
                id="title"
                name="title"
                class=INPUT_CLASS
                prop:value=titleX
                on:input=move |ev| {
                    titleX.set(event_target_value(&ev));
                }
            />

            <label for="content">Content</label>
            <textarea
                type="text"
                autocomplete="off"
                id="content"
                name="content"
                class=TEXTAREA_CLASS
                rows="3"
                prop:value=contentX
                on:input=move |ev| {
                    contentX.set(event_target_value(&ev));
                }
            >
            </textarea>
            <button
                type="submit"
                class="w-full relative inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground shadow hover:bg-primary/90 h-9 px-4 py-3"
            >
                <span>Create New</span>
                <span class="absolute right-5">
                    <Spinner fetching=loading/>
                </span>
            </button>
        </form>
    }
}

const INPUT_CLASS: &str = "flex w-full rounded-md border-input shadow-sm focus:border-primary focus:ring focus:ring-primary/50 focus:ring-opacity-20 bg-transparent text-sm";
const TEXTAREA_CLASS: &str = "flex min-h-[60px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus:border-primary focus:ring focus:ring-primary/50 focus:ring-opacity-20 disabled:cursor-not-allowed disabled:opacity-50";

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct AllTodosKey;

fn all_todos_query() -> QueryScope<AllTodosKey, Vec<Todo>> {
    create_query(
        move |_| async move {
            gloo_timers::future::sleep(Duration::from_millis(1000)).await;
            TODOS.with_borrow(|todos| todos.clone())
        },
        QueryOptions::default(),
    )
}

async fn add_todo(title: String, content: String) -> Todo {
    gloo_timers::future::sleep(Duration::from_millis(1000)).await;
    let new_id = TODOS.with_borrow(|todos| todos.last().map(|t| t.id.0 + 1).unwrap_or(1));
    let todo = Todo {
        id: TodoId(new_id),
        title,
        content,
        completed: false,
    };

    TODOS.with_borrow_mut(|todos| {
        todos.push(todo.clone());
    });

    todo
}

async fn set_completed(id: TodoId, completed: bool) -> bool {
    gloo_timers::future::sleep(Duration::from_millis(1000)).await;
    TODOS.with_borrow_mut(|todos| {
        if let Some(todo) = todos.iter_mut().find(|todo| todo.id == id) {
            todo.completed = completed;
        }
    });

    true
}

async fn delete_todo(id: TodoId) -> bool {
    gloo_timers::future::sleep(Duration::from_millis(1000)).await;
    TODOS.with_borrow_mut(|todos| {
        todos.retain(|todo| todo.id != id);
    });

    true
}

thread_local! {
    static TODOS: RefCell<Vec<Todo>> = RefCell::new(Vec::new());
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Todo {
    id: TodoId,
    title: String,
    content: String,
    completed: bool,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TodoId(u32);
