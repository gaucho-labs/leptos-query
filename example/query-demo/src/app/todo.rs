use std::rc::Rc;

use crate::repo::*;

use leptos::*;
use leptos_query::*;
use leptos_router::{ActionForm, MultiActionForm};
use serde::{Deserialize, Serialize};

#[component]
pub fn TodoPage() -> impl IntoView {
    provide_context(TodoState {
        selected_todo: RwSignal::new(None),
    });

    view! {
        <div class="flex items-center w-full font-medium gap-2">
            // <h1 class="font-semibold ml-3 text-lg">Leptos Query Todos</h1>
            <div class="flex flex-col items-center gap-2">
                <AddTodoEntry/>
                <TodoList/>
            </div>
            <SelectedTodo/>
        </div>
    }
}

#[derive(Clone, Debug)]
struct TodoState {
    selected_todo: RwSignal<Option<TodoId>>,
}

fn use_todo_state() -> TodoState {
    use_context::<TodoState>().expect("TodoState")
}

#[component]
fn SelectedTodo() -> impl IntoView {
    let state = use_todo_state();

    let todo_id = state.selected_todo;

    move || {
        let todo_id = todo_id.get();
        if let Some(todo_id) = todo_id {
            let result = todo_query().use_query(move || todo_id.clone());
            let todo = Signal::derive(move || result.data.get().and_then(|r| r.ok()).flatten());

            view! {
                <Transition>
                    {move || {
                        todo.get()
                            .map(|todo| {
                                view! {
                                    <div class="flex flex-col items-start justify-between w-full rounded-xl border bg-card text-card-foreground shadow p-6 gap-2">
                                        <h2 class="font-semibold text-lg">Selected Todo</h2>
                                        <div class="flex flex-col items-center gap-2">
                                            <div class="text-base">{todo.title}</div>
                                            <div class="line-clamp-1 text-muted-foreground text-sm">
                                                {todo.description}
                                            </div>
                                        </div>
                                    </div>
                                }
                            })
                    }}

                </Transition>
            }
            .into_view()
        } else {
            view! {
                <div
                    type="button"
                    class="relative block w-full h-full rounded-lg border-2 border-border border-dashed p-12 text-center hover:border-border/50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
                >
                    <span class="mt-2 block text-sm font-semibold text-foreground">
                        No Todo Selected
                    </span>
                </div>
            }.into_view()
        }
    }
}

#[component]
pub fn TodoList() -> impl IntoView {
    let result = all_todos_query().use_query(|| AllTodos);
    let todos = Signal::derive(move || result.data.get().and_then(|r| r.ok()).unwrap_or_default());

    view! {
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            <ul class="w-72 gap-2">

                <For
                    each=todos
                    key=|todo| todo.id
                    children=move |todo| {
                        view! { <TodoListItem todo=todo/> }
                    }
                />

            </ul>
        </Transition>
    }
}

#[component]
fn TodoListItem(todo: Todo) -> impl IntoView {
    let selected_todo = use_todo_state().selected_todo;

    let delete_todo = create_server_action_with_callbacks::<DeleteTodo>(
        {
            let todo_query = todo_query();
            let all_todos = all_todos_query();

            move |input| {
                let id = input.id;
                all_todos.cancel_query(AllTodos);
                todo_query.set_query_data(TodoId(id), Ok(None));
                // TODO: This causes a signal disposed of warning.
                all_todos.update_query_data_mut(AllTodos, move |todos| {
                    if let Ok(todos) = todos {
                        todos.retain(|t| t.id != id);
                    }
                });
            }
        },
        move |(_, result)| {
            let todo_query = todo_query();
            let all_todos = all_todos_query();
            if let Ok(_) = result {
                all_todos.invalidate_query(AllTodos);
                todo_query.invalidate_query(TodoId(todo.id));
            }
        },
    );

    view! {
        <li
            class="flex flex-col items-start justify-between w-full rounded-xl border bg-card text-card-foreground shadow p-6 gap-2"
            on:click=move |_| { selected_todo.set(Some(TodoId(todo.id))) }
        >
            <div class="text-base">{todo.title}</div>
            <div class="line-clamp-1 text-muted-foreground text-sm">{todo.description}</div>
            <ActionForm action=delete_todo>
                <input type="hidden" name="id" value=todo.id/>
                <button
                    type="submit"
                    value="Delete"
                    class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90 h-9 px-4 py-2"
                    on:click=|event| { event.stop_propagation() }
                >
                    Delete
                </button>
            </ActionForm>
        </li>
    }
}

#[component]
fn AddTodoEntry() -> impl IntoView {
    let todo_query = todo_query();
    let all_todos = all_todos_query();

    let form_ref = create_node_ref::<html::Form>();

    let add_todo = create_server_multi_action_with_callbacks::<AddTodo>(
        |_| {},
        move |(_, result)| {
            if let Ok(todo) = result {
                form_ref.get_untracked().expect("Form Node").reset();

                all_todos.cancel_query(AllTodos);
                all_todos.update_query_data_mut(AllTodos, |todos| {
                    if let Ok(todos) = todos {
                        todos.push(todo.clone());
                    }
                });
                todo_query.set_query_data(TodoId(todo.id), Ok(Some(todo.clone())));

                todo_query.invalidate_query(TodoId(todo.id));
                all_todos.invalidate_query(AllTodos);
            }
        },
    );

    view! {
        <MultiActionForm
            node_ref=form_ref
            action=add_todo
            class="flex flex-col items-start gap-2 w-72 bg-card 100 p-4 rounded-md border"
        >
            <label for="title">Title</label>
            <input
                type="text"
                autocomplete="off"
                id="title"
                name="create[title]"
                class="text-sm block w-full rounded-md border-border shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
            />
            <label for="description">Description</label>
            <textarea
                type="text"
                autocomplete="off"
                id="description"
                name="create[description]"
                class="text-sm mt-1 block w-full rounded-md border-border shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
            ></textarea>
            <button
                type="submit"
                class="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground shadow hover:bg-primary/90 h-9 px-4 py-2"
            >
                Create New
            </button>
        </MultiActionForm>
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct AllTodos;

fn all_todos_query() -> QueryScope<AllTodos, Result<Vec<Todo>, ServerFnError>> {
    create_query(|_| get_todos(), QueryOptions::default())
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct TodoId(u16);
fn todo_query() -> QueryScope<TodoId, Result<Option<Todo>, ServerFnError>> {
    create_query(|id: TodoId| get_todo(id.0), QueryOptions::default())
}

pub fn create_server_action_with_callbacks<S>(
    on_invoke: impl Fn(&S) + 'static,
    on_settled: impl Fn((S, &Result<S::Output, ServerFnError<S::Error>>)) + 'static,
) -> Action<S, Result<S::Output, ServerFnError<S::Error>>>
where
    S: Clone + server_fn::ServerFn,
    S::Error: Clone,
{
    let on_invoke = Rc::new(on_invoke);
    let on_settled = Rc::new(on_settled);

    // The server is able to call the function directly
    #[cfg(feature = "ssr")]
    let action_function = move |args: &S| {
        let args = args.clone();
        let on_invoke = on_invoke.clone();
        let on_settled = on_settled.clone();

        async move {
            on_invoke(&args);
            let result = S::run_body(args.clone()).await;
            on_settled((args, &result));
            result
        }
    };

    // When not on the server send a fetch to request the fn call.
    #[cfg(not(feature = "ssr"))]
    let action_function = move |args: &S| {
        let args = args.clone();
        let on_invoke = on_invoke.clone();
        let on_settled = on_settled.clone();

        async move {
            on_invoke(&args);
            let result = S::run_on_client(args.clone()).await;
            on_settled((args, &result));
            result
        }
    };

    // create the action
    Action::new(action_function).using_server_fn()
}

pub fn create_server_multi_action_with_callbacks<S>(
    on_invoke: impl Fn(&S) + 'static,
    on_settled: impl Fn((S, &Result<S::Output, ServerFnError<S::Error>>)) + 'static,
) -> MultiAction<S, Result<S::Output, ServerFnError<S::Error>>>
where
    S: Clone + server_fn::ServerFn,
{
    let on_invoke = Rc::new(on_invoke);
    let on_settled = Rc::new(on_settled);
    #[cfg(feature = "ssr")]
    let c = move |args: &S| {
        let args = args.clone();
        let on_invoke = on_invoke.clone();
        let on_settled = on_settled.clone();
        async move {
            on_invoke(&args);
            let result = S::run_body(args.clone()).await;
            on_settled((args, &result));
            result
        }
    };
    #[cfg(not(feature = "ssr"))]
    let c = move |args: &S| {
        let args = args.clone();
        let on_invoke = on_invoke.clone();
        let on_settled = on_settled.clone();
        async move {
            on_invoke(&args);
            let result = S::run_on_client(args.clone()).await;
            on_settled((args, &result));
            result
        }
    };

    create_multi_action(c).using_server_fn::<S>()
}
