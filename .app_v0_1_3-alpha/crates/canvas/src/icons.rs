use egui::{Color32, Rect, Ui};
use egui_lucide::Lucide;
use polygram_core::types::NodeKind;

/// Нарисовать иконку Lucide на экране egui. `rect` задаёт размер и положение.
pub fn paint_lucide(ui: &Ui, rect: Rect, icon: Lucide, color: Color32) {
    let image = icon.color(color).size(rect.width()).image();
    image.paint_at(ui, rect);
}

/// Вернуть иконку Lucide для обложки объекта.
/// Если `custom` не пусто — использовать его (имя варианта Lucide).
/// Иначе — иконка по умолчанию для вида объекта: Процесс → BookUp2, Отдел → GraphHub.
pub fn default_icon_for(kind: NodeKind, custom: &str) -> Lucide {
    if !custom.is_empty() {
        return lucide_by_name(custom);
    }
    match kind {
        NodeKind::Process => Lucide::BookUp2,
        NodeKind::Department => Lucide::GraphHub,
        _ => Lucide::BookUp2,
    }
}

/// Сопоставить строковое имя варианта Lucide с вариантом enum.
pub fn lucide_by_name(name: &str) -> Lucide {
    match name {
        "BookUp2" => Lucide::BookUp2,
        "GraphHub" => Lucide::GraphHub,
        "Activity" => Lucide::Activity,
        "Archive" => Lucide::Archive,
        "Bell" => Lucide::Bell,
        "Book" => Lucide::Book,
        "Box" => Lucide::Box,
        "Briefcase" => Lucide::Briefcase,
        "Calendar" => Lucide::Calendar,
        "Camera" => Lucide::Camera,
        "Check" => Lucide::Check,
        "Circle" => Lucide::Circle,
        "Clock" => Lucide::Clock,
        "Code2" => Lucide::Code,
        "Command" => Lucide::Command,
        "Database" => Lucide::Database,
        "DollarSign" => Lucide::DollarSign,
        "FileText" => Lucide::FileText,
        "Flag" => Lucide::Flag,
        "Globe" => Lucide::Globe,
        "Headphones" => Lucide::Headphones,
        "Key" => Lucide::Key,
        "Layers" => Lucide::Layers,
        "Mail" => Lucide::Mail,
        "Megaphone" => Lucide::Megaphone,
        "MessageCircle" => Lucide::MessageCircle,
        "Phone" => Lucide::Phone,
        "Shield" => Lucide::Shield,
        "ShoppingBag" => Lucide::ShoppingBag,
        "ShoppingCart" => Lucide::ShoppingCart,
        "Square" => Lucide::Square,
        "Star" => Lucide::Star,
        "Target" => Lucide::Target,
        "Trash" => Lucide::Trash,
        "Triangle" => Lucide::Triangle,
        "Truck" => Lucide::Truck,
        "User" => Lucide::User,
        "Users" => Lucide::Users,
        "Zap" => Lucide::Zap,
        _ => Lucide::BookUp2,
    }
}
