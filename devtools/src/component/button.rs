use super::*;
use leptos::*;

#[component]
pub fn Button(
    children: ChildrenFn,
    color: ColorOption,
    #[prop(attrs)] attributes: Vec<(&'static str, Attribute)>,
) -> impl IntoView {
    match color {
        ColorOption::Blue => view! {
            <button
                type="button"
                class="text-white bg-blue-700 hover:bg-blue-800 focus:outline-none focus:ring-4 focus:ring-blue-300 font-medium rounded-full text-xs px-2 py-1 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
                {..attributes}
            >
                {children}
            </button>
        },
        ColorOption::Green => view! {
            <button
                type="button"
                class="text-white bg-green-700 hover:bg-green-800 focus:outline-none focus:ring-4 focus:ring-green-300 font-medium rounded-full text-xs px-2 py-1 text-center  dark:bg-green-600 dark:hover:bg-green-700 dark:focus:ring-green-800"
                {..attributes}
            >
                {children}
            </button>
        },
        ColorOption::Red => view! {
            <button
                type="button"
                class="text-white bg-red-700 hover:bg-red-800 focus:outline-none focus:ring-4 focus:ring-red-300 font-medium rounded-full text-xs px-2 py-1 text-center  dark:bg-red-600 dark:hover:bg-red-700 dark:focus:ring-red-900"
                {..attributes}
            >
                {children}
            </button>
        },
        ColorOption::Yellow => view! {
            <button
                type="button"
                class="text-white bg-yellow-400 hover:bg-yellow-500 focus:outline-none focus:ring-4 focus:ring-yellow-300 font-medium rounded-full text-xs px-2 py-1 text-center  dark:focus:ring-yellow-900"
                {..attributes}
            >
                {children}
            </button>
        },
        ColorOption::Gray => view! {
            <button
                type="button"
                class="text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-xs px-2 py-1  dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:hover:border-gray-600 dark:focus:ring-gray-700"
                {..attributes}
            >
                {children}
            </button>
        },
    }
}
