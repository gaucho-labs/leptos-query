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
                <span class="lq-inline-flex lq-items-center lq-gap-x-1.5 lq-rounded-md lq-bg-blue-100 lq-px-2 lq-py-1 lq-text-xs lq-font-medium lq-text-blue-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="lq-h-1.5 lq-w-1.5 lq-fill-blue-500"
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
                <span class="lq-inline-flex lq-items-center lq-gap-x-1.5 lq-rounded-md lq-bg-green-100 lq-px-2 lq-py-1 lq-text-xs lq-font-medium lq-text-green-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="lq-h-1.5 lq-w-1.5 lq-fill-green-500"
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
                <span class="lq-inline-flex lq-items-center lq-gap-x-1.5 lq-rounded-md lq-bg-red-100 lq-px-2 lq-py-1 lq-text-xs lq-font-medium lq-text-red-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="lq-h-1.5 lq-w-1.5 lq-fill-red-500"
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
                <span class="lq-inline-flex lq-items-center lq-gap-x-1.5 lq-rounded-md lq-bg-gray-100 lq-px-2 lq-py-1 lq-text-xs lq-font-medium lq-text-gray-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="lq-h-1.5 lq-w-1.5 lq-fill-gray-500"
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
                <span class="lq-inline-flex lq-items-center lq-gap-x-1.5 lq-rounded-md lq-bg-yellow-100 lq-px-2 lq-py-1 lq-text-xs lq-font-medium lq-text-yellow-700">
                    {if dot {
                        Some(
                            view! {
                                <svg
                                    class="lq-h-1.5 lq-w-1.5 lq-fill-yellow-500"
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
