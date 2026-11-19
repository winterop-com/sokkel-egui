//! Every project, the one edited last first, as the web's `ProjectsPage.tsx`: each a card with its
//! still, its length and when it was edited, opened in the editor, renamed, duplicated or deleted.

use eframe::egui::Ui;
use sokkel_egui::{Button, ButtonSize, ButtonVariant, Empty, Icon, MenuItem, MetaLine, PageHeader};
use sokkel_egui_media::{MediaCard, MediaGrid, MediaShape};

use crate::app::CutApp;
use crate::backend::{clips_count, counted, project_length};
use crate::editor::thumbnail_of;
use crate::media::draw_source;
use crate::model::Project;

/// What the projects page was asked this frame, each by the project's id.
#[derive(Clone, Debug, Default)]
pub struct ProjectsAsked {
    pub create: bool,
    pub open: Option<String>,
    pub rename: Option<String>,
    pub duplicate: Option<String>,
    pub delete: Option<String>,
}

/// A project card's menu: Open, Rename, Duplicate, and Delete after a rule.
fn card_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::action("Open").icon(Icon::ExternalLink),
        MenuItem::action("Rename").icon(Icon::PencilLine),
        MenuItem::action("Duplicate").icon(Icon::Copy),
        MenuItem::Separator,
        MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
    ]
}

pub fn projects_page(ui: &mut Ui, projects: &[&Project], now: i64) -> ProjectsAsked {
    let mut asked = ProjectsAsked::default();
    PageHeader::new("Projects")
        .count(counted(projects.len(), "project", "projects"))
        .description("Edits cut from the clips in the media bin. Changes are saved as they are made.")
        .show(ui, |ui| {
            asked.create = ui
                .add(Button::new("New").icon(Icon::Plus).size(ButtonSize::Sm))
                .clicked();
        });
    if projects.is_empty() {
        Empty::new("No projects")
            .description("Make a project to cut clips from the media bin into an edit.")
            .show(ui, |ui| {
                asked.create |= ui
                    .add(
                        Button::new("New project")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
            });
        return asked;
    }
    let menu = card_menu();
    MediaGrid::new(MediaShape::Wide).show(ui, projects.len(), |ui, index| {
        let project = projects[index];
        let still = thumbnail_of(project);
        let subtitle = MetaLine::new([clips_count(project), CutApp::edited(project.updated, now)]).text();
        let mut card = MediaCard::new(("project", &project.id), &project.name)
            .shape(MediaShape::Wide)
            .subtitle(subtitle)
            .badge(project_length(project))
            .menu(&menu);
        card = match still {
            Some((source, at)) => card.cover(move |painter, rect| draw_source(painter, rect, source, at)),
            None => card.placeholder_seed(&project.id),
        };
        let out = card.show(ui);
        let id = Some(project.id.clone());
        if out.selected {
            asked.open = id.clone();
        }
        match out.menu {
            Some(0) => asked.open = id,
            Some(1) => asked.rename = id,
            Some(2) => asked.duplicate = id,
            Some(4) => asked.delete = id,
            _ => {}
        }
    });
    asked
}
