use android_activity::AndroidApp;
use filmcraft_engine::Session;
use filmcraft_ui_egui::FilmcraftApp;
use jni::objects::JValue;
use std::path::{Path, PathBuf};

const WORKSPACE_DIR: &str = "/sdcard/Download/ArtCraft";

fn ensure_workspace_dir() -> PathBuf {
    let p = PathBuf::from(WORKSPACE_DIR);
    let _ = std::fs::create_dir_all(&p);
    p
}

fn apply_immersive_sticky(app: &AndroidApp) {
    let vm_ptr = app.vm_as_ptr();
    let activity_ptr = app.activity_as_ptr();
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return;
    }

    unsafe {
        let jvm = match jni::JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) {
            Ok(jvm) => jvm,
            Err(_) => return,
        };
        let mut env = match jvm.attach_current_thread() {
            Ok(env) => env,
            Err(_) => return,
        };
        let activity = match jni::objects::JObject::from_raw(activity_ptr as jni::sys::jobject) {
            activity if !activity.as_raw().is_null() => activity,
            _ => return,
        };

        if let Ok(window) = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[]) {
            if let Ok(window_obj) = window.l() {
                if let Ok(decor_view) = env.call_method(&window_obj, "getDecorView", "()Landroid/view/View;", &[]) {
                    if let Ok(decor_obj) = decor_view.l() {
                        let flags: i32 = 4096 | 4 | 2 | 1024 | 512 | 256;
                        let _ = env.call_method(&decor_obj, "setSystemUiVisibility", "(I)V", &[JValue::Int(flags)]);
                    }
                }
            }
        }
    }
}

struct FilmAndroidApp {
    app: FilmcraftApp,
}

impl eframe::App for FilmAndroidApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.app.raw_input_hook(ctx, raw_input);
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
    }

    fn on_exit(&mut self) {
        self.app.on_exit();
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("FilmCraft"),
    );

    apply_immersive_sticky(&app);
    let _ = ensure_workspace_dir();

    let mut options = eframe::NativeOptions::default();
    options.android_app = Some(app);

    let _ = eframe::run_native(
        "FilmCraft",
        options,
        Box::new(|cc| {
            let session = Session::default();
            let mut app = FilmcraftApp::new(session);

            if let Some(rs) = cc.wgpu_render_state.clone() {
                app.set_wgpu(rs);
            }

            // Set up Android storage hooks
            app.hooks.pick_files = Some(Box::new(|exts: &[&str]| {
                let base_dir = ensure_workspace_dir();
                let mut candidates = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&base_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                        if path.is_file() && (exts.is_empty() || exts.iter().any(|&e| e.eq_ignore_ascii_case(&ext))) {
                            candidates.push(path.to_string_lossy().into_owned());
                        }
                    }
                }
                candidates
            }));

            app.hooks.pick_save = Some(Box::new(|name: &str| {
                let base_dir = ensure_workspace_dir();
                let path = Path::new(name);
                let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| name.to_string());
                let mut target = base_dir.join(file_name);
                if target.extension().is_none() {
                    target.set_extension("fcproj");
                }
                Some(target.to_string_lossy().into_owned())
            }));

            app.hooks.pick_save_as = Some(Box::new(|_filter: &str, exts: &[&str], name: &str| {
                let base_dir = ensure_workspace_dir();
                let path = Path::new(name);
                let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| name.to_string());
                let mut target = base_dir.join(file_name);
                if target.extension().is_none() && !exts.is_empty() {
                    target.set_extension(exts[0]);
                }
                Some(target.to_string_lossy().into_owned())
            }));

            app.hooks.pick_folder = Some(Box::new(|| {
                Some(ensure_workspace_dir().to_string_lossy().into_owned())
            }));

            Ok(Box::new(FilmAndroidApp { app }))
        }),
    );
}
