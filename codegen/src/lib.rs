mod parse;

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Expr, LitStr};

// noinspection SpellCheckingInspection
/// For documentation on how to use this, visit [`SupportedFields`]
///
/// [`SupportedFields`]: ./struct.SupportedFields.html
#[proc_macro]
pub fn arcdps_export(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as parse::ArcDpsGen);
    let sig = input.sig;
    let build = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is not set") + "\0";
    let build = LitStr::new(build.as_str(), Span::call_site());
    let (raw_name, span) = if let Some(input_name) = input.name {
        let name = input_name.value();
        (name, input_name.span())
    } else {
        let name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME is not set");
        (name, Span::call_site())
    };
    let name = LitStr::new(raw_name.as_str(), span);
    let out_name = raw_name + "\0";
    let out_name = LitStr::new(out_name.as_str(), span);

    let (abstract_combat, cb_combat) = build_combat(input.raw_combat, input.combat);
    let (abstract_combat_local, cb_combat_local) =
        build_combat_local(input.raw_combat_local, input.combat_local);
    let (abstract_imgui, cb_imgui) = build_imgui(input.raw_imgui, input.imgui);
    let (abstract_options_end, cb_options_end) =
        build_options_end(input.raw_options_end, input.options_end);
    let (abstract_options_windows, cb_options_windows) =
        build_options_windows(input.raw_options_windows, input.options_windows);
    let (abstract_wnd_filter, cb_wnd_filter) =
        build_wnd_filter(input.raw_wnd_filter, input.wnd_filter);
    let (abstract_wnd_nofilter, cb_wnd_nofilter) =
        build_wnd_nofilter(input.raw_wnd_nofilter, input.wnd_nofilter);

    let export = quote! {
        ArcDpsExport {
            size: ::std::mem::size_of::<ArcDpsExport>(),
            sig: #sig,
            imgui_version: 18000,
            out_build: #build.as_ptr(),
            out_name: #out_name.as_ptr(),
            combat: #cb_combat,
            combat_local: #cb_combat_local,
            imgui: #cb_imgui,
            options_end: #cb_options_end,
            options_windows: #cb_options_windows,
            wnd_filter: #cb_wnd_filter,
            wnd_nofilter: #cb_wnd_nofilter,
        }
    };

    let init = if let Some(init) = input.init {
        let span = syn::Error::new_spanned(&init, "").span();
        quote_spanned! (span => unsafe { (#init as InitFunc)(__SWAPCHAIN) })
    } else {
        quote! {Ok(())}
    };

    let release = if let Some(release) = input.release {
        let span = syn::Error::new_spanned(&release, "").span();
        quote_spanned! (span => (#release as ReleaseFunc)())
    } else {
        quote! {}
    };

    let (abstract_extras_squad_update, extras_squad_update) = build_extras_squad_update(
        input.raw_unofficial_extras_squad_update,
        input.unofficial_extras_squad_update,
    );
    let (abstract_extras_chat_message, extras_chat_message) = build_extras_chat_message(
        input.raw_unofficial_extras_chat_message,
        input.unofficial_extras_chat_message,
    );
    let (abstract_extras_chat_message2, extras_chat_message2) = build_extras_chat_message2(
        input.raw_unofficial_extras_chat_message2,
        input.unofficial_extras_chat_message2,
    );
    let abstract_extras_init = build_extras_init(
        input.raw_unofficial_extras_init,
        input.unofficial_extras_init,
        extras_squad_update,
        extras_chat_message,
        extras_chat_message2,
        &out_name,
    );

    #[cfg(feature = "imgui")]
    let sys_init = quote! {
        use ::arcdps::imgui;

        #[no_mangle]
        // export -- arcdps looks for this exported function and calls the address it returns on client load
        // if you need any of the ignored values, create an issue with your use case
        pub unsafe extern "system" fn get_init_addr(
            __arc_version: *mut c_char,
            __imguictx: *mut imgui::sys::ImGuiContext,
            __id3dptr: *mut c_void,
            __arc_dll: *mut c_void,
            __mallocfn: Option<unsafe extern "C" fn(sz: usize, user_data: *mut c_void) -> *mut c_void>,
            __freefn: Option<unsafe extern "C" fn(ptr: *mut c_void, user_data: *mut c_void)>,
        ) -> unsafe extern "system" fn() -> *const ArcDpsExport {
            imgui::sys::igSetCurrentContext(__imguictx);
            imgui::sys::igSetAllocatorFunctions(__mallocfn, __freefn, ::core::ptr::null_mut());
            __CTX = Some(imgui::Context::current());
            let __ctx = &raw const __CTX;
            __UI = Some(imgui::Ui::from_ctx((*__ctx).as_ref().unwrap()));
            __SWAPCHAIN = NonNull::new(__id3dptr);
            ::arcdps::__init(__arc_version, __arc_dll, #name);
            __load
        }

        static mut __CTX: Option<imgui::Context> = None;
        static mut __UI: Option<imgui::Ui> = None;
    };

    #[cfg(not(feature = "imgui"))]
    let sys_init = quote! {
        #[no_mangle]
        // export -- arcdps looks for this exported function and calls the address it returns on client load
        // if you need any of the ignored values, create an issue with your use case
        pub unsafe extern "system" fn get_init_addr(
            __arc_version: *mut c_char,
            _imguictx: *mut c_void,
            __id3dptr: *mut c_void,
            __arc_dll: *mut c_void,
            _mallocfn: Option<unsafe extern "C" fn(sz: usize, user_data: *mut c_void) -> *mut c_void>,
            _freefn: Option<unsafe extern "C" fn(ptr: *mut c_void, user_data: *mut c_void)>,
        ) -> unsafe extern "system" fn() -> *const ArcDpsExport {
            __SWAPCHAIN = NonNull::new(__id3dptr);
            ::arcdps::__init(__arc_version, __arc_dll, #name);
            __load
        }
    };

    let res = quote! {
        mod __arcdps_gen_export {
            use super::*;
            use ::std::os::raw::{c_char, c_void};
            use ::std::ptr::NonNull;
            use ::arcdps::ArcDpsExport;
            use ::arcdps::{InitFunc, ReleaseFunc};

            #abstract_combat
            #abstract_combat_local
            #abstract_imgui
            #abstract_options_end
            #abstract_options_windows
            #abstract_wnd_filter
            #abstract_wnd_nofilter
            #abstract_extras_squad_update
            #abstract_extras_chat_message
            #abstract_extras_chat_message2
            #abstract_extras_init

            static __EXPORT: ArcDpsExport = #export;
            static mut __EXPORT_ERROR: ArcDpsExport = ArcDpsExport {
                    size: 0,
                    sig: 0,
                    imgui_version: 18000,
                    out_build: #build.as_ptr(),
                    out_name: #out_name.as_ptr(),
                    combat: None,
                    combat_local: None,
                    imgui: None,
                    options_end: None,
                    options_windows: None,
                    wnd_filter: None,
                    wnd_nofilter: None,
                };
            static mut __ERROR_STRING: String = String::new();
            static mut __SWAPCHAIN: Option<NonNull<c_void>> = None;

            unsafe extern "system" fn __load() -> *const ArcDpsExport {
                let mut __export = &raw const __EXPORT;
                let __res: Result<(), Box<dyn ::std::error::Error>> = #init;
                if let Err(__e) = __res {
                    unsafe {
                        __ERROR_STRING = __e.to_string() + "\0";
                        __EXPORT_ERROR.size = &raw const __ERROR_STRING as _;
                        __export = &raw const __EXPORT_ERROR;
                    }
                }

                __export
            }

            unsafe extern "system" fn __unload() {
                #release
            }

            #sys_init

            #[no_mangle]
            /* export -- arcdps looks for this exported function and calls the address it returns on client exit */
            pub extern "system" fn get_release_addr() -> unsafe extern "system" fn() {
                __unload
            }
        }
    };
    res.into()
}

fn build_extras_squad_update(
    raw: Option<Expr>,
    safe: Option<Expr>,
) -> (TokenStream, Option<TokenStream>) {
    let mut abstract_wrapper = quote! {};
    let cb_safe = match (raw, safe) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            Some(quote_spanned!(span => Some(#raw as _) ))
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_wrapper = quote_spanned!(span =>
            unsafe extern "C" fn __abstract_extras_squad_update(__users: *const ::arcdps::RawUserInfo, __count: u64) {
                let _ = #safe as ::arcdps::ExtrasSquadUpdateCallback;
                let __users = ::std::slice::from_raw_parts(__users, __count as _);
                let __users = __users.iter().map(::arcdps::helpers::convert_extras_user as ::arcdps::UserConvert);
                #safe(__users)
            });
            Some(
                quote_spanned!(span => Some(__arcdps_gen_export::__abstract_extras_squad_update as _) ),
            )
        }
        _ => None,
    };
    (abstract_wrapper, cb_safe)
}

fn build_extras_chat_message(
    raw: Option<Expr>,
    safe: Option<Expr>,
) -> (TokenStream, Option<TokenStream>) {
    let mut abstract_wrapper = quote! {};
    let cb_safe = match (raw, safe) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            Some(quote_spanned!(span => Some(#raw as _) ))
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_wrapper = quote_spanned!(span =>
            unsafe extern "C" fn __abstract_extras_chat_message(__msg: *const ::arcdps::RawSquadMessageInfo) {
                let _ = #safe as ::arcdps::ExtrasChatMessageCallback;
                let __msg = ::arcdps::helpers::convert_extras_squad_chat_message(&*__msg);
                #safe(&__msg)
            });
            Some(
                quote_spanned!(span => Some(__arcdps_gen_export::__abstract_extras_chat_message as _) ),
            )
        }
        _ => None,
    };
    (abstract_wrapper, cb_safe)
}

fn build_extras_chat_message2(
    raw: Option<Expr>,
    safe: Option<Expr>,
) -> (TokenStream, Option<TokenStream>) {
    let mut abstract_wrapper = quote! {};
    let cb_safe = match (raw, safe) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            Some(quote_spanned!(span => Some(#raw as _) ))
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_wrapper = quote_spanned!(span =>
                unsafe extern "C" fn __abstract_extras_chat_message2(__msg_type: ::arcdps::ChatMessageType, __msg: ::arcdps::RawChatMessageInfo2) {
                let _ = #safe as ::arcdps::ExtrasChatMessage2Callback;
                let __msg = ::arcdps::helpers::convert_extras_chat_message2(__msg_type, __msg);
                #safe(&__msg)
            });
            Some(
                quote_spanned!(span => Some(__arcdps_gen_export::__abstract_extras_chat_message2 as _) ),
            )
        }
        _ => None,
    };
    (abstract_wrapper, cb_safe)
}

fn build_extras_init(
    raw: Option<Expr>,
    safe: Option<Expr>,
    squad_update: Option<TokenStream>,
    chat_message: Option<TokenStream>,
    chat_message2: Option<TokenStream>,
    name: &LitStr,
) -> TokenStream {
    let needs_init = squad_update.is_some() || chat_message.is_some();
    let squad_cb = squad_update.unwrap_or(quote! { None });
    let chat_cb = chat_message.unwrap_or(quote! { None });
    let chat_cb2 = chat_message2.unwrap_or(quote! { None });

    let basic_init = quote!(
        if __addon.api_version != 2 {
            return;
        }
        if __addon.max_info_version < 1 {
            return;
        }

        fn __fill_v1(__sub: *mut ::arcdps::RawExtrasSubscriberInfo<::arcdps::InfoV1>) {
            let __sub = unsafe { &mut *__sub };
            __sub.header.info_version = 1;

            __sub.subscriber_name = #name.as_ptr();
            __sub.squad_update_callback = #squad_cb;
            __sub.language_changed_callback = None;
            __sub.key_bind_changed_callback = None;
        }

        fn __fill_v2(__sub: *mut ::arcdps::RawExtrasSubscriberInfo<::arcdps::InfoV2>) {
            __fill_v1(__sub.cast());

            let __sub = unsafe { &mut *__sub };
            __sub.header.info_version = 2;

            __sub.chat_message_callback = #chat_cb;
        }

        fn __fill_v3(__sub: *mut ::arcdps::RawExtrasSubscriberInfo<::arcdps::InfoV3>) {
            __fill_v2(__sub.cast());

            let __sub = unsafe { &mut *__sub };
            __sub.header.info_version = 3;

            __sub.chat_message_callback2 = #chat_cb2;
        }

        match __addon.max_info_version {
            1 => __fill_v1(__sub.cast()),
            2 => __fill_v2(__sub.cast()),
            _ => __fill_v3(__sub.cast()),
        }
    );

    let abstract_wrapper = match (raw, safe) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            quote_spanned!(span =>
                let _ = #raw as ::arcdps::RawExtrasSubscriberInitSignature;

                #raw(__addon, __sub)
            )
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            quote_spanned!(span =>
                #basic_init

                let _ = #safe as ::arcdps::ExtrasInitFunc;
                let __user = ::arcdps::helpers::get_str_from_pc_char(__addon.self_account_name as _)
                                .map(|n| n.trim_start_matches(':'));
                let __version = ::arcdps::helpers::get_str_from_pc_char(__addon.string_version as _);

                #safe(__user, __version)
            )
        }
        _ if needs_init => basic_init,
        _ => return quote! {},
    };
    use syn::spanned::Spanned;
    quote_spanned!(abstract_wrapper.span() =>
        #[no_mangle]
        unsafe extern "system" fn arcdps_unofficial_extras_subscriber_init(
                                    __addon: &::arcdps::RawExtrasAddonInfo,
                                    __sub: *mut ::arcdps::RawExtrasSubscriberInfoHeader
        ) {
            #abstract_wrapper
        }
    )
}

fn build_wnd_filter(raw_wnd: Option<Expr>, wnd: Option<Expr>) -> (TokenStream, TokenStream) {
    build_wnd(raw_wnd, wnd, quote! { __abstract_wnd_filter })
}

fn build_wnd_nofilter(raw_wnd: Option<Expr>, wnd: Option<Expr>) -> (TokenStream, TokenStream) {
    build_wnd(raw_wnd, wnd, quote! { __abstract_wnd_nofilter })
}

fn build_wnd(
    raw_wnd_filter: Option<Expr>,
    wnd_filter: Option<Expr>,
    func_name: TokenStream,
) -> (TokenStream, TokenStream) {
    let mut abstract_wnd_filter = quote! {};
    let cb_wnd_filter = match (raw_wnd_filter, wnd_filter) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            quote_spanned!(span => Some(#raw as _) )
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_wnd_filter = quote_spanned!(span =>
            unsafe extern "C" fn #func_name (_h_wnd: *mut c_void, __u_msg: u32,
                    __w_param: usize, __l_param: isize
                ) -> u32 {
                let _ = #safe as ::arcdps::WndProcCallback;
                use ::arcdps::{WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP};
                match __u_msg {
                    WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP => {
                        let __key_down = __u_msg & 1 == 0;
                        let __prev_key_down = (__l_param >> 30) & 1 == 1;

                        if #safe(__w_param, __key_down, __prev_key_down)
                        {
                            __u_msg
                        } else {
                            0
                        }
                    },
                    _ => __u_msg,
                }
            });
            quote_spanned!(span => Some(__arcdps_gen_export::#func_name as _) )
        }
        _ => quote! { None },
    };
    (abstract_wnd_filter, cb_wnd_filter)
}

fn build_options_windows(
    raw_options_windows: Option<Expr>,
    options_windows: Option<Expr>,
) -> (TokenStream, TokenStream) {
    let mut abstract_options_windows = quote! {};
    let cb_options_windows = match (raw_options_windows, options_windows) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            quote_spanned!(span => Some(#raw as _) )
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_options_windows = quote_spanned!(span =>
            unsafe extern "C" fn __abstract_options_windows(__window_name: *mut c_char) -> bool {
                let _ = #safe as ::arcdps::OptionsWindowsCallback;
                let __ui = &raw const __UI;
                let __ui = (*__ui).as_ref().unwrap();
                #safe(__ui, ::arcdps::helpers::get_str_from_pc_char(__window_name))
            });
            quote_spanned!(span => Some(__arcdps_gen_export::__abstract_options_windows as _) )
        }
        _ => quote! { None },
    };
    (abstract_options_windows, cb_options_windows)
}

fn build_options_end(
    raw_options_end: Option<Expr>,
    options_end: Option<Expr>,
) -> (TokenStream, TokenStream) {
    let mut abstract_options_end = quote! {};
    let cb_options_end = match (raw_options_end, options_end) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            quote_spanned!(span => Some(#raw as _) )
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_options_end = quote_spanned!(span =>
            unsafe extern "C" fn __abstract_options_end() {
                let _ = #safe as ::arcdps::OptionsCallback;
                let __ui = &raw const __UI;
                let __ui = (*__ui).as_ref().unwrap();
                #safe(__ui)
            });
            quote_spanned!(span => Some(__arcdps_gen_export::__abstract_options_end as _) )
        }
        _ => quote! { None },
    };
    (abstract_options_end, cb_options_end)
}

fn build_imgui(raw_imgui: Option<Expr>, imgui: Option<Expr>) -> (TokenStream, TokenStream) {
    let mut abstract_imgui = quote! {};
    let cb_imgui = match (raw_imgui, imgui) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            quote_spanned!(span => Some(#raw as _) )
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_imgui = quote_spanned!(span =>
            unsafe extern "C" fn __abstract_imgui(__loading: u32) {
                let _ = #safe as ::arcdps::ImguiCallback;
                let __ui = &raw const __UI;
                let __ui = (*__ui).as_ref().unwrap();
                #safe(__ui, __loading != 0)
            });
            quote_spanned!(span => Some(__arcdps_gen_export::__abstract_imgui as _) )
        }
        _ => quote! { None },
    };
    (abstract_imgui, cb_imgui)
}

fn build_combat_local(
    raw_combat: Option<Expr>,
    combat: Option<Expr>,
) -> (TokenStream, TokenStream) {
    build_cbt(raw_combat, combat, quote! { __abstract_combat_local })
}

fn build_combat(raw_combat: Option<Expr>, combat: Option<Expr>) -> (TokenStream, TokenStream) {
    build_cbt(raw_combat, combat, quote! { __abstract_combat })
}

fn build_cbt(
    raw_combat: Option<Expr>,
    combat: Option<Expr>,
    func_name: TokenStream,
) -> (TokenStream, TokenStream) {
    let mut abstract_combat = quote! {};
    let cb_combat = match (raw_combat, combat) {
        (Some(raw), _) => {
            let span = syn::Error::new_spanned(&raw, "").span();
            quote_spanned!(span => Some(#raw as _) )
        }
        (_, Some(safe)) => {
            let span = syn::Error::new_spanned(&safe, "").span();
            abstract_combat = quote_spanned!(span =>
            unsafe extern "C" fn #func_name(
                    __ev: Option<&::arcdps::CombatEvent>,
                    __src: Option<&::arcdps::RawAgent>,
                    __dst: Option<&::arcdps::RawAgent>,
                    __skill_name: *mut c_char,
                    __id: u64,
                    __revision: u64,
                ) {
                    let _ = #safe as ::arcdps::CombatCallback;
                    let __args = ::arcdps::helpers::get_combat_args_from_raw(__ev, __src, __dst, __skill_name);
                    #safe(__args.ev, __args.src, __args.dst, __args.skill_name, __id, __revision)
            });
            quote_spanned!(span => Some(__arcdps_gen_export::#func_name as _) )
        }
        _ => quote! { None },
    };
    (abstract_combat, cb_combat)
}
