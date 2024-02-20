use leptos::*;

use super::ColorOption;

#[component]
pub fn DotBadge(
    children: ChildrenFn,
    color: ColorOption,
    #[prop(default = true)] dot: bool,
) -> impl IntoView {
    match color {
        ColorOption::Blue => {
            view! {
                <span class="inline-flex items-center gap-x-1.5 rounded-md bg-blue-100 px-2 py-1 text-xs font-medium text-blue-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="h-1.5 w-1.5 fill-blue-500"
                                    viewBox="0 0 6 6"
                                    aria-hidden="true"
                                >
                                    <circle cx="3" cy="3" r="3"></circle>
                                </svg>
                            },
                        )
                    } else {
                        None
                    }}
                    {children}
                </span>
            }
        }
        ColorOption::Green => {
            view! {
                <span class="inline-flex items-center gap-x-1.5 rounded-md bg-green-100 px-2 py-1 text-xs font-medium text-green-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="h-1.5 w-1.5 fill-green-500"
                                    viewBox="0 0 6 6"
                                    aria-hidden="true"
                                >
                                    <circle cx="3" cy="3" r="3"></circle>
                                </svg>
                            },
                        )
                    } else {
                        None
                    }}
                    {children}
                </span>
            }
        }
        ColorOption::Red => {
            view! {
                <span class="inline-flex items-center gap-x-1.5 rounded-md bg-red-100 px-2 py-1 text-xs font-medium text-red-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="h-1.5 w-1.5 fill-red-500"
                                    viewBox="0 0 6 6"
                                    aria-hidden="true"
                                >
                                    <circle cx="3" cy="3" r="3"></circle>
                                </svg>
                            },
                        )
                    } else {
                        None
                    }}
                    {children}
                </span>
            }
        }
        ColorOption::Gray => {
            view! {
                <span class="inline-flex items-center gap-x-1.5 rounded-md bg-gray-100 px-2 py-1 text-xs font-medium text-gray-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="h-1.5 w-1.5 fill-gray-500"
                                    viewBox="0 0 6 6"
                                    aria-hidden="true"
                                >
                                    <circle cx="3" cy="3" r="3"></circle>
                                </svg>
                            },
                        )
                    } else {
                        None
                    }}
                    {children}
                </span>
            }
        }
        ColorOption::Yellow => {
            view! {
                <span class="inline-flex items-center gap-x-1.5 rounded-md bg-yellow-100 px-2 py-1 text-xs font-medium text-yellow-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="h-1.5 w-1.5 fill-yellow-500"
                                    viewBox="0 0 6 6"
                                    aria-hidden="true"
                                >
                                    <circle cx="3" cy="3" r="3"></circle>
                                </svg>
                            },
                        )
                    } else {
                        None
                    }}
                    {children}
                </span>
            }
        }
    }
}
