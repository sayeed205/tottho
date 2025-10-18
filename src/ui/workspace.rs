//! Tottho Workspace
//! 
//! Main workspace implementation following Zed's workspace pattern,
//! integrating all UI systems into a cohesive interface.

use std::sync::{Arc, Mutex};
use gpui::{Context, IntoElement, Render, div, Styled, InteractiveElement, Window, ParentElement};
use crate::ui::{
    ThemeEngine, LayoutEngine, PanelManager, CommandPalette, KeyboardNavigator,
    UIError, UIResult
};

/// Main Tottho workspace
pub struct TotthoWorkspace {
    theme_engine: Arc<ThemeEngine>,
    layout_engine: Arc<LayoutEngine>,
    panel_manager: Arc<Mutex<PanelManager>>,
    command_palette: Arc<Mutex<CommandPalette>>,
    keyboard_navigator: Arc<Mutex<KeyboardNavigator>>,
}

impl TotthoWorkspace {
    /// Create a new Tottho workspace
    pub fn new(
        theme_engine: Arc<ThemeEngine>,
        layout_engine: Arc<LayoutEngine>,
        panel_manager: Arc<Mutex<PanelManager>>,
        command_palette: Arc<Mutex<CommandPalette>>,
        keyboard_navigator: Arc<Mutex<KeyboardNavigator>>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            theme_engine,
            layout_engine,
            panel_manager,
            command_palette,
            keyboard_navigator,
        }
    }
    
    /// Get the theme engine
    pub fn theme_engine(&self) -> &Arc<ThemeEngine> {
        &self.theme_engine
    }
    
    /// Get the layout engine
    pub fn layout_engine(&self) -> &Arc<LayoutEngine> {
        &self.layout_engine
    }
    
    /// Get the panel manager
    pub fn panel_manager(&self) -> &Arc<Mutex<PanelManager>> {
        &self.panel_manager
    }
    
    /// Get the command palette
    pub fn command_palette(&self) -> &Arc<Mutex<CommandPalette>> {
        &self.command_palette
    }
    
    /// Get the keyboard navigator
    pub fn keyboard_navigator(&self) -> &Arc<Mutex<KeyboardNavigator>> {
        &self.keyboard_navigator
    }
}

impl Render for TotthoWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme_engine.active_theme();
        
        div()
            .id("tottho-workspace")
            .size_full()
            .bg(theme.colors().background)
            .text_color(theme.colors().text)
            .flex()
            .flex_col()
            .child(
                // Title bar area
                div()
                    .id("title-bar")
                    .w_full()
                    .h_8()
                    .bg(theme.colors().title_bar_background)
                    .flex()
                    .items_center()
                    .px_4()
                    .child("Tottho - Database Management Tool")
            )
            .child(
                // Main content area
                div()
                    .id("workspace-content")
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_4()
                            .child("🗄️ Tottho Database Manager")
                            .child("UI Foundation Ready")
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.colors().text_muted)
                                    .child("Press Ctrl+Shift+P to open command palette")
                            )
                    )
            )
    }
}