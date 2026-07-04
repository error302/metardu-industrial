// The contents of this file are automatically generated and should not be modified directly.  See the `build` directory.

use crate::{
    CodePair,
    Color,
    Drawing,
    DrawingItem,
    DrawingItemMut,
    DxfError,
    DxfResult,
    ExtensionGroup,
    Handle,
    LwPolylineVertex,
    Point,
    Vector,
    XData,
};
use crate::code_pair_put_back::CodePairPutBack;
use crate::code_pair_writer::CodePairWriter;
use crate::extension_data;
use crate::helper_functions::*;
use crate::tables::*;
use crate::x_data;

use crate::enums::*;
use crate::enum_primitive::FromPrimitive;
use crate::objects::*;

use std::io::{Read, Write};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct EntityCommon {
    pub handle: Handle,
    pub extension_data_groups: Vec<ExtensionGroup>,
    pub x_data: Vec<XData>,
    #[doc(hidden)]
    pub __owner_handle: Handle,
    pub is_in_paper_space: bool,
    pub layer: String,
    pub line_type_name: String,
    pub elevation: f64,
    #[doc(hidden)]
    pub __material_handle: Handle,
    pub color: Color,
    pub lineweight_enum_value: i16,
    pub line_type_scale: f64,
    pub is_visible: bool,
    pub image_byte_count: i32,
    pub preview_image_data: Vec<Vec<u8>>,
    pub color_24_bit: i32,
    pub color_name: String,
    pub transparency: i32,
    #[doc(hidden)]
    pub __plot_style_handle: Handle,
    pub shadow_mode: ShadowMode,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Entity {
    pub common: EntityCommon,
    pub specific: EntityType,
}

impl Default for EntityCommon {
    fn default() -> EntityCommon {
        EntityCommon {
            handle: Handle::empty(),
            extension_data_groups: vec![],
            x_data: vec![],
            __owner_handle: Handle::empty(),
            is_in_paper_space: false,
            layer: String::from("0"),
            line_type_name: String::from("BYLAYER"),
            elevation: 0.0,
            __material_handle: Handle::empty(),
            color: Color::by_layer(),
            lineweight_enum_value: 0,
            line_type_scale: 1.0,
            is_visible: true,
            image_byte_count: 0,
            preview_image_data: vec![],
            color_24_bit: 0,
            color_name: String::new(),
            transparency: 0,
            __plot_style_handle: Handle::empty(),
            shadow_mode: ShadowMode::CastsAndReceivesShadows,
        }
    }
}

impl EntityCommon {
    pub fn get_owner<'a>(&self, drawing: &'a Drawing) -> Option<DrawingItem<'a>> {
        drawing.get_item_by_handle(self.__owner_handle)
    }
    pub fn set_owner(&mut self, item: &DrawingItemMut) {
        self.__owner_handle = item.get_handle();
    }
    pub fn get_material<'a>(&self, drawing: &'a Drawing) -> Option<&'a Object> {
        match drawing.get_item_by_handle(self.__material_handle) {
            Some(DrawingItem::Object(val)) => {
                match val.specific {
                    ObjectType::Material(_) => Some(val),
                    _ => None,
                }
            },
            _ => None,
        }
    }
    pub fn set_material(&mut self, item: &Object) -> DxfResult<()> {
        match item.specific {
            ObjectType::Material { .. } => self.__material_handle = item.common.handle,
            _ => return Err(DxfError::WrongItemType),
        }

        Ok(())
    }
    pub fn get_plot_style<'a>(&self, drawing: &'a Drawing) -> Option<DrawingItem<'a>> {
        drawing.get_item_by_handle(self.__plot_style_handle)
    }
    pub fn set_plot_style(&mut self, item: &DrawingItemMut) {
        self.__plot_style_handle = item.get_handle();
    }
    pub(crate) fn apply_individual_pair<I>(&mut self, pair: &CodePair, iter: &mut CodePairPutBack<I>) -> DxfResult<()>
        where I: Read {

        match pair.code {
            5 => { self.handle = pair.as_handle()? },
            extension_data::EXTENSION_DATA_GROUP => {
                let group = ExtensionGroup::read_group(pair.assert_string()?, iter, pair.offset)?;
                self.extension_data_groups.push(group);
            },
            330 => { self.__owner_handle = pair.as_handle()? },
            67 => { self.is_in_paper_space = as_bool(pair.assert_i16()?) },
            8 => { self.layer = pair.assert_string()? },
            6 => { self.line_type_name = pair.assert_string()? },
            38 => { self.elevation = pair.assert_f64()? },
            347 => { self.__material_handle = pair.as_handle()? },
            62 => { self.color = Color::from_raw_value(pair.assert_i16()?) },
            370 => { self.lineweight_enum_value = pair.assert_i16()? },
            48 => { self.line_type_scale = pair.assert_f64()? },
            60 => { self.is_visible = !as_bool(pair.assert_i16()?) },
            92 => { self.image_byte_count = pair.assert_i32()? },
            310 => { self.preview_image_data.push(pair.assert_binary()?) },
            420 => { self.color_24_bit = pair.assert_i32()? },
            430 => { self.color_name = pair.assert_string()? },
            440 => { self.transparency = pair.assert_i32()? },
            390 => { self.__plot_style_handle = pair.as_handle()? },
            284 => { self.shadow_mode = enum_from_number!(ShadowMode, CastsAndReceivesShadows, from_i16, pair.assert_i16()?) },
            x_data::XDATA_APPLICATIONNAME => {
                let x = XData::read_item(pair.assert_string()?, iter)?;
                self.x_data.push(x);
            },
            _ => (), // unknown code, just ignore
        }
        Ok(())
    }
    pub(crate) fn write<T>(&self, version: AcadVersion, write_handles: bool, writer: &mut CodePairWriter<T>) -> DxfResult<()>
        where T: Write + ?Sized {

        let ent = self;
        if write_handles {
            writer.write_code_pair(&CodePair::new_string(5, &self.handle.as_string()))?;
        }
        if version >= AcadVersion::R14 {
            for group in &self.extension_data_groups {
                group.write(writer)?;
            }
        }
        if write_handles && ent.__owner_handle != Handle(0) {
            writer.write_code_pair(&CodePair::new_string(330, &ent.__owner_handle.as_string()))?;
        }
        writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbEntity")))?;
        if version >= AcadVersion::R12 && ent.is_in_paper_space != false {
            writer.write_code_pair(&CodePair::new_i16(67, as_i16(ent.is_in_paper_space)))?;
        }
        writer.write_code_pair(&CodePair::new_string(8, &ent.layer))?;
        if ent.line_type_name != String::from("BYLAYER") {
            writer.write_code_pair(&CodePair::new_string(6, &ent.line_type_name))?;
        }
        if version <= AcadVersion::R12 && ent.elevation != 0.0 {
            writer.write_code_pair(&CodePair::new_f64(38, ent.elevation))?;
        }
        if version >= AcadVersion::R2007 {
            writer.write_code_pair(&CodePair::new_string(347, &ent.__material_handle.as_string()))?;
        }
        if ent.color != Color::by_layer() {
            writer.write_code_pair(&CodePair::new_i16(62, ent.color.get_raw_value()))?;
        }
        if version >= AcadVersion::R2000 {
            writer.write_code_pair(&CodePair::new_i16(370, ent.lineweight_enum_value))?;
        }
        if version >= AcadVersion::R13 && ent.line_type_scale != 1.0 {
            writer.write_code_pair(&CodePair::new_f64(48, ent.line_type_scale))?;
        }
        if version >= AcadVersion::R13 && ent.is_visible != true {
            writer.write_code_pair(&CodePair::new_i16(60, as_i16(!ent.is_visible)))?;
        }
        if version >= AcadVersion::R2000 && ent.image_byte_count != 0 {
            writer.write_code_pair(&CodePair::new_i32(92, ent.image_byte_count))?;
        }
        if version >= AcadVersion::R2000 {
            for v in &ent.preview_image_data {
                writer.write_code_pair(&CodePair::new_binary(310, v.clone()))?;
            }
        }
        if version >= AcadVersion::R2004 && ent.color_24_bit != 0 {
            writer.write_code_pair(&CodePair::new_i32(420, ent.color_24_bit))?;
        }
        if version >= AcadVersion::R2004 {
            writer.write_code_pair(&CodePair::new_string(430, &ent.color_name))?;
        }
        if version >= AcadVersion::R2004 {
            writer.write_code_pair(&CodePair::new_i32(440, ent.transparency))?;
        }
        if version >= AcadVersion::R2007 {
            writer.write_code_pair(&CodePair::new_string(390, &ent.__plot_style_handle.as_string()))?;
        }
        if version >= AcadVersion::R2007 {
            writer.write_code_pair(&CodePair::new_i16(284, ent.shadow_mode as i16))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub enum EntityType {
    Face3D(Face3D),
    Solid3D(Solid3D),
    ProxyEntity(ProxyEntity),
    Arc(Arc),
    ArcAlignedText(ArcAlignedText),
    AttributeDefinition(AttributeDefinition),
    Attribute(Attribute),
    Body(Body),
    Circle(Circle),
    RotatedDimension(RotatedDimension),
    RadialDimension(RadialDimension),
    DiameterDimension(DiameterDimension),
    AngularThreePointDimension(AngularThreePointDimension),
    OrdinateDimension(OrdinateDimension),
    Ellipse(Ellipse),
    Helix(Helix),
    Image(Image),
    Insert(Insert),
    Leader(Leader),
    Light(Light),
    Line(Line),
    LwPolyline(LwPolyline),
    MLine(MLine),
    MText(MText),
    OleFrame(OleFrame),
    Ole2Frame(Ole2Frame),
    ModelPoint(ModelPoint),
    Polyline(Polyline),
    Ray(Ray),
    Region(Region),
    RText(RText),
    Section(Section),
    Seqend(Seqend),
    Shape(Shape),
    Solid(Solid),
    Spline(Spline),
    Text(Text),
    Tolerance(Tolerance),
    Trace(Trace),
    DgnUnderlay(DgnUnderlay),
    DwfUnderlay(DwfUnderlay),
    PdfUnderlay(PdfUnderlay),
    Vertex(Vertex),
    Wipeout(Wipeout),
    XLine(XLine),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Face3D {
    pub first_corner: Point,
    pub second_corner: Point,
    pub third_corner: Point,
    pub fourth_corner: Point,
    pub edge_flags: i32,
}

impl Default for Face3D {
    fn default() -> Face3D {
        Face3D {
            first_corner: Point::origin(),
            second_corner: Point::origin(),
            third_corner: Point::origin(),
            fourth_corner: Point::origin(),
            edge_flags: 0,
        }
    }
}

impl Face3D {
    pub fn get_is_first_edge_invisible(&self) -> bool {
        self.edge_flags & 1 != 0
    }
    pub fn set_is_first_edge_invisible(&mut self, val: bool) {
        if val {
            self.edge_flags |= 1;
        }
        else {
            self.edge_flags &= !1;
        }
    }
    pub fn get_is_second_edge_invisible(&self) -> bool {
        self.edge_flags & 2 != 0
    }
    pub fn set_is_second_edge_invisible(&mut self, val: bool) {
        if val {
            self.edge_flags |= 2;
        }
        else {
            self.edge_flags &= !2;
        }
    }
    pub fn get_is_third_edge_invisible(&self) -> bool {
        self.edge_flags & 4 != 0
    }
    pub fn set_is_third_edge_invisible(&mut self, val: bool) {
        if val {
            self.edge_flags |= 4;
        }
        else {
            self.edge_flags &= !4;
        }
    }
    pub fn get_is_fourth_edge_invisible(&self) -> bool {
        self.edge_flags & 8 != 0
    }
    pub fn set_is_fourth_edge_invisible(&mut self, val: bool) {
        if val {
            self.edge_flags |= 8;
        }
        else {
            self.edge_flags &= !8;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Solid3D {
    pub format_version_number: i16,
    pub custom_data: Vec<String>,
    pub custom_data2: Vec<String>,
    #[doc(hidden)]
    pub __history_object_handle: Handle,
}

impl Default for Solid3D {
    fn default() -> Solid3D {
        Solid3D {
            format_version_number: 1,
            custom_data: vec![],
            custom_data2: vec![],
            __history_object_handle: Handle::empty(),
        }
    }
}

impl Solid3D {
    pub fn get_history_object<'a>(&self, drawing: &'a Drawing) -> Option<DrawingItem<'a>> {
        drawing.get_item_by_handle(self.__history_object_handle)
    }
    pub fn set_history_object(&mut self, item: &DrawingItemMut) {
        self.__history_object_handle = item.get_handle();
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct ProxyEntity {
    pub proxy_entity_class_id: i32,
    pub application_entity_class_id: i32,
    pub graphics_data_size: i32,
    pub graphics_data_string: Vec<Vec<u8>>,
    pub entity_data_size: i32,
    pub entity_data_string: Vec<Vec<u8>>,
    pub object_id_1: Vec<String>,
    pub object_id_2: Vec<String>,
    pub object_id_3: Vec<String>,
    pub object_id_4: Vec<String>,
    pub terminator: i32,
    #[doc(hidden)]
    pub __object_drawing_format: u32,
    pub original_data_format_is_dxf: bool,
}

impl Default for ProxyEntity {
    fn default() -> ProxyEntity {
        ProxyEntity {
            proxy_entity_class_id: 498,
            application_entity_class_id: 500,
            graphics_data_size: 0,
            graphics_data_string: vec![],
            entity_data_size: 0,
            entity_data_string: vec![],
            object_id_1: vec![],
            object_id_2: vec![],
            object_id_3: vec![],
            object_id_4: vec![],
            terminator: 0,
            __object_drawing_format: 0,
            original_data_format_is_dxf: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Arc {
    pub thickness: f64,
    pub center: Point,
    pub radius: f64,
    pub normal: Vector,
    /// Arc start angle in degrees.
    pub start_angle: f64,
    /// Arc end angle in degrees.
    pub end_angle: f64,
}

impl Default for Arc {
    fn default() -> Arc {
        Arc {
            thickness: 0.0,
            center: Point::origin(),
            radius: 0.0,
            normal: Vector::z_axis(),
            start_angle: 0.0,
            end_angle: 360.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct ArcAlignedText {
    pub text: String,
    pub font_name: String,
    pub bigfont_name: String,
    pub text_style_name: String,
    pub center_point: Point,
    pub arc_radius: f64,
    pub width_factor: f64,
    pub text_height: f64,
    pub character_spacing: f64,
    pub offset_from_arc: f64,
    pub right_offset: f64,
    pub left_offset: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub is_character_order_reversed: bool,
    pub direction_flag: i16,
    pub alignment_flag: i16,
    pub side_flag: i16,
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underline: bool,
    pub character_set_value: i16,
    pub pitch_and_family_value: i16,
    pub font_type: FontType,
    pub color_index: i32,
    pub extrusion_direction: Vector,
    pub wizard_flag: i16,
}

impl Default for ArcAlignedText {
    fn default() -> ArcAlignedText {
        ArcAlignedText {
            text: String::new(),
            font_name: String::new(),
            bigfont_name: String::new(),
            text_style_name: String::new(),
            center_point: Point::origin(),
            arc_radius: 0.0,
            width_factor: 1.0,
            text_height: 0.0,
            character_spacing: 0.0,
            offset_from_arc: 0.0,
            right_offset: 0.0,
            left_offset: 0.0,
            start_angle: 0.0,
            end_angle: 0.0,
            is_character_order_reversed: false,
            direction_flag: 0,
            alignment_flag: 0,
            side_flag: 0,
            is_bold: false,
            is_italic: false,
            is_underline: false,
            character_set_value: 0,
            pitch_and_family_value: 0,
            font_type: FontType::TTF,
            color_index: 0,
            extrusion_direction: Vector::z_axis(),
            wizard_flag: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct AttributeDefinition {
    pub thickness: f64,
    pub location: Point,
    pub text_height: f64,
    pub value: String,
    pub rotation: f64,
    pub relative_x_scale_factor: f64,
    pub oblique_angle: f64,
    pub text_style_name: String,
    pub text_generation_flags: i32,
    pub horizontal_text_justification: HorizontalTextJustification,
    pub second_alignment_point: Point,
    pub normal: Vector,
    pub version: Version,
    pub prompt: String,
    pub text_tag: String,
    pub flags: i32,
    pub field_length: i16,
    pub vertical_text_justification: VerticalTextJustification,
    pub is_locked_in_block: bool,
    pub keep_duplicate_records: bool,
    pub m_text_flag: MTextFlag,
    pub is_really_locked: bool,
    #[doc(hidden)]
    pub __secondary_attribute_count: i32,
    #[doc(hidden)]
    pub __secondary_attributes_handle: Vec<Handle>,
    pub alignment_point: Point,
    pub annotation_scale: f64,
    pub x_record_tag: String,
    pub m_text: MText,
}

impl Default for AttributeDefinition {
    fn default() -> AttributeDefinition {
        AttributeDefinition {
            thickness: 0.0,
            location: Point::origin(),
            text_height: 1.0,
            value: String::new(),
            rotation: 0.0,
            relative_x_scale_factor: 1.0,
            oblique_angle: 0.0,
            text_style_name: String::from("STANDARD"),
            text_generation_flags: 0,
            horizontal_text_justification: HorizontalTextJustification::Left,
            second_alignment_point: Point::origin(),
            normal: Vector::z_axis(),
            version: Version::R2010,
            prompt: String::new(),
            text_tag: String::new(),
            flags: 0,
            field_length: 0,
            vertical_text_justification: VerticalTextJustification::Baseline,
            is_locked_in_block: false,
            keep_duplicate_records: false,
            m_text_flag: MTextFlag::MultilineAttribute,
            is_really_locked: false,
            __secondary_attribute_count: 0,
            __secondary_attributes_handle: vec![],
            alignment_point: Point::origin(),
            annotation_scale: 1.0,
            x_record_tag: String::new(),
            m_text: MText::default(),
        }
    }
}

impl AttributeDefinition {
    pub fn get_is_text_backwards(&self) -> bool {
        self.text_generation_flags & 2 != 0
    }
    pub fn set_is_text_backwards(&mut self, val: bool) {
        if val {
            self.text_generation_flags |= 2;
        }
        else {
            self.text_generation_flags &= !2;
        }
    }
    pub fn get_is_text_upside_down(&self) -> bool {
        self.text_generation_flags & 4 != 0
    }
    pub fn set_is_text_upside_down(&mut self, val: bool) {
        if val {
            self.text_generation_flags |= 4;
        }
        else {
            self.text_generation_flags &= !4;
        }
    }
    pub fn get_is_invisible(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_invisible(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_constant(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_constant(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_is_input_verification_required(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_is_input_verification_required(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_is_attribute_present(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_is_attribute_present(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_secondary_attributes<'a>(&self, drawing: &'a Drawing) -> Vec<DrawingItem<'a>> {
        self.__secondary_attributes_handle.iter().filter_map(|&h| drawing.get_item_by_handle(h)).collect()
    }
    pub fn add_secondary_attributes(&mut self, item: &DrawingItemMut) {
        self.__secondary_attributes_handle.push(item.get_handle());
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Attribute {
    pub thickness: f64,
    pub location: Point,
    pub text_height: f64,
    pub value: String,
    pub version: Version,
    pub attribute_tag: String,
    pub flags: i32,
    pub field_length: i16,
    pub rotation: f64,
    pub relative_x_scale_factor: f64,
    pub oblique_angle: f64,
    pub text_style_name: String,
    pub text_generation_flags: i32,
    pub horizontal_text_justification: HorizontalTextJustification,
    pub vertical_text_justification: VerticalTextJustification,
    pub second_alignment_point: Point,
    pub normal: Vector,
    pub is_locked_in_block: bool,
    pub keep_duplicate_records: bool,
    pub m_text_flag: MTextFlag,
    pub is_really_locked: bool,
    #[doc(hidden)]
    pub __secondary_attribute_count: i32,
    #[doc(hidden)]
    pub __secondary_attributes_handle: Vec<Handle>,
    pub alignment_point: Point,
    pub annotation_scale: f64,
    pub x_record_tag: String,
    pub m_text: MText,
}

impl Default for Attribute {
    fn default() -> Attribute {
        Attribute {
            thickness: 0.0,
            location: Point::origin(),
            text_height: 1.0,
            value: String::new(),
            version: Version::R2010,
            attribute_tag: String::new(),
            flags: 0,
            field_length: 0,
            rotation: 0.0,
            relative_x_scale_factor: 1.0,
            oblique_angle: 0.0,
            text_style_name: String::from("STANDARD"),
            text_generation_flags: 0,
            horizontal_text_justification: HorizontalTextJustification::Left,
            vertical_text_justification: VerticalTextJustification::Baseline,
            second_alignment_point: Point::origin(),
            normal: Vector::z_axis(),
            is_locked_in_block: false,
            keep_duplicate_records: false,
            m_text_flag: MTextFlag::MultilineAttribute,
            is_really_locked: false,
            __secondary_attribute_count: 0,
            __secondary_attributes_handle: vec![],
            alignment_point: Point::origin(),
            annotation_scale: 1.0,
            x_record_tag: String::new(),
            m_text: MText::default(),
        }
    }
}

impl Attribute {
    pub fn get_is_invisible(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_invisible(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_constant(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_constant(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_is_input_verification_required(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_is_input_verification_required(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_is_attribute_present(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_is_attribute_present(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_is_text_backwards(&self) -> bool {
        self.text_generation_flags & 2 != 0
    }
    pub fn set_is_text_backwards(&mut self, val: bool) {
        if val {
            self.text_generation_flags |= 2;
        }
        else {
            self.text_generation_flags &= !2;
        }
    }
    pub fn get_is_text_upside_down(&self) -> bool {
        self.text_generation_flags & 4 != 0
    }
    pub fn set_is_text_upside_down(&mut self, val: bool) {
        if val {
            self.text_generation_flags |= 4;
        }
        else {
            self.text_generation_flags &= !4;
        }
    }
    pub fn get_secondary_attributes<'a>(&self, drawing: &'a Drawing) -> Vec<DrawingItem<'a>> {
        self.__secondary_attributes_handle.iter().filter_map(|&h| drawing.get_item_by_handle(h)).collect()
    }
    pub fn add_secondary_attributes(&mut self, item: &DrawingItemMut) {
        self.__secondary_attributes_handle.push(item.get_handle());
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Body {
    pub format_version_number: i16,
    pub custom_data: Vec<String>,
    pub custom_data2: Vec<String>,
}

impl Default for Body {
    fn default() -> Body {
        Body {
            format_version_number: 1,
            custom_data: vec![],
            custom_data2: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Circle {
    pub thickness: f64,
    pub center: Point,
    pub radius: f64,
    pub normal: Vector,
}

impl Default for Circle {
    fn default() -> Circle {
        Circle {
            thickness: 0.0,
            center: Point::origin(),
            radius: 0.0,
            normal: Vector::z_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct DimensionBase {
    pub version: Version,
    pub block_name: String,
    pub definition_point_1: Point,
    pub text_mid_point: Point,
    pub dimension_type: DimensionType,
    pub attachment_point: AttachmentPoint,
    pub text_line_spacing_style: TextLineSpacingStyle,
    pub text_line_spacing_factor: f64,
    pub actual_measurement: f64,
    pub text: String,
    pub text_rotation_angle: f64,
    pub horizontal_direction_angle: f64,
    pub normal: Vector,
    pub dimension_style_name: String,
    pub is_block_reference_referenced_by_this_block_only: bool,
    pub is_ordinate_x_type: bool,
    pub is_at_user_defined_location: bool,
}

impl Default for DimensionBase {
    fn default() -> DimensionBase {
        DimensionBase {
            version: Version::R2010,
            block_name: String::from("*MODEL_SPACE"),
            definition_point_1: Point::origin(),
            text_mid_point: Point::origin(),
            dimension_type: DimensionType::Aligned,
            attachment_point: AttachmentPoint::TopLeft,
            text_line_spacing_style: TextLineSpacingStyle::AtLeast,
            text_line_spacing_factor: 1.0,
            actual_measurement: 0.0,
            text: String::from("<>"),
            text_rotation_angle: 0.0,
            horizontal_direction_angle: 0.0,
            normal: Vector::z_axis(),
            dimension_style_name: String::from("STANDARD"),
            is_block_reference_referenced_by_this_block_only: false,
            is_ordinate_x_type: false,
            is_at_user_defined_location: false,
        }
    }
}

impl DimensionBase {
    pub(crate) fn write<T>(&self, version: AcadVersion, writer: &mut CodePairWriter<T>) -> DxfResult<()>
        where T: Write + ?Sized {

        let ent = self;
        writer.write_code_pair(&CodePair::new_str(100, "AcDbDimension"))?;
        if version >= AcadVersion::R2010 {
            writer.write_code_pair(&CodePair::new_i16(280, ent.version as i16))?;
        }
        writer.write_code_pair(&CodePair::new_string(2, &ent.block_name))?;
        writer.write_code_pair(&CodePair::new_f64(10, ent.definition_point_1.x))?;
        writer.write_code_pair(&CodePair::new_f64(20, ent.definition_point_1.y))?;
        writer.write_code_pair(&CodePair::new_f64(30, ent.definition_point_1.z))?;
        writer.write_code_pair(&CodePair::new_f64(11, ent.text_mid_point.x))?;
        writer.write_code_pair(&CodePair::new_f64(21, ent.text_mid_point.y))?;
        writer.write_code_pair(&CodePair::new_f64(31, ent.text_mid_point.z))?;
        writer.write_code_pair(&CodePair::new_i16(70, self.get_dimension_type()))?;
        if version >= AcadVersion::R2000 {
            writer.write_code_pair(&CodePair::new_i16(71, ent.attachment_point as i16))?;
        }
        if version >= AcadVersion::R2000 && ent.text_line_spacing_style != TextLineSpacingStyle::AtLeast {
            writer.write_code_pair(&CodePair::new_i16(72, ent.text_line_spacing_style as i16))?;
        }
        if version >= AcadVersion::R2000 && ent.text_line_spacing_factor != 1.0 {
            writer.write_code_pair(&CodePair::new_f64(41, ent.text_line_spacing_factor))?;
        }
        if version >= AcadVersion::R2000 && ent.actual_measurement != 0.0 {
            writer.write_code_pair(&CodePair::new_f64(42, ent.actual_measurement))?;
        }
        writer.write_code_pair(&CodePair::new_string(1, &ent.text))?;
        if ent.text_rotation_angle != 0.0 {
            writer.write_code_pair(&CodePair::new_f64(53, ent.text_rotation_angle))?;
        }
        if ent.horizontal_direction_angle != 0.0 {
            writer.write_code_pair(&CodePair::new_f64(51, ent.horizontal_direction_angle))?;
        }
        if ent.normal != Vector::z_axis() {
            writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
            writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
            writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
        }
        if version >= AcadVersion::R12 {
            writer.write_code_pair(&CodePair::new_string(3, &ent.dimension_style_name))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct RotatedDimension {
    pub dimension_base: DimensionBase,
    pub insertion_point: Point,
    pub definition_point_2: Point,
    pub definition_point_3: Point,
    pub rotation_angle: f64,
    pub extension_line_angle: f64,
}

impl Default for RotatedDimension {
    fn default() -> RotatedDimension {
        RotatedDimension {
            dimension_base: Default::default(),
            insertion_point: Point::origin(),
            definition_point_2: Point::origin(),
            definition_point_3: Point::origin(),
            rotation_angle: 0.0,
            extension_line_angle: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct RadialDimension {
    pub dimension_base: DimensionBase,
    pub definition_point_2: Point,
    pub leader_length: f64,
}

impl Default for RadialDimension {
    fn default() -> RadialDimension {
        RadialDimension {
            dimension_base: Default::default(),
            definition_point_2: Point::origin(),
            leader_length: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct DiameterDimension {
    pub dimension_base: DimensionBase,
    pub definition_point_2: Point,
    pub leader_length: f64,
}

impl Default for DiameterDimension {
    fn default() -> DiameterDimension {
        DiameterDimension {
            dimension_base: Default::default(),
            definition_point_2: Point::origin(),
            leader_length: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct AngularThreePointDimension {
    pub dimension_base: DimensionBase,
    pub definition_point_2: Point,
    pub definition_point_3: Point,
    pub definition_point_4: Point,
    pub definition_point_5: Point,
}

impl Default for AngularThreePointDimension {
    fn default() -> AngularThreePointDimension {
        AngularThreePointDimension {
            dimension_base: Default::default(),
            definition_point_2: Point::origin(),
            definition_point_3: Point::origin(),
            definition_point_4: Point::origin(),
            definition_point_5: Point::origin(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct OrdinateDimension {
    pub dimension_base: DimensionBase,
    pub definition_point_2: Point,
    pub definition_point_3: Point,
}

impl Default for OrdinateDimension {
    fn default() -> OrdinateDimension {
        OrdinateDimension {
            dimension_base: Default::default(),
            definition_point_2: Point::origin(),
            definition_point_3: Point::origin(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Ellipse {
    pub center: Point,
    pub major_axis: Vector,
    pub normal: Vector,
    pub minor_axis_ratio: f64,
    /// Ellipse start angle in radians.
    pub start_parameter: f64,
    /// Ellipse end angle in radians.
    pub end_parameter: f64,
}

impl Default for Ellipse {
    fn default() -> Ellipse {
        Ellipse {
            center: Point::origin(),
            major_axis: Vector::x_axis(),
            normal: Vector::z_axis(),
            minor_axis_ratio: 1.0,
            start_parameter: 0.0,
            end_parameter: ::std::f64::consts::PI * 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Helix {
    pub major_release_number: i32,
    pub maintainence_release_number: i32,
    pub axis_base_point: Point,
    pub start_point: Point,
    pub axis_vector: Vector,
    pub radius: f64,
    pub number_of_turns: f64,
    pub turn_height: f64,
    pub is_right_handed: bool,
    pub constraint: HelixConstraint,
}

impl Default for Helix {
    fn default() -> Helix {
        Helix {
            major_release_number: 0,
            maintainence_release_number: 0,
            axis_base_point: Point::origin(),
            start_point: Point::origin(),
            axis_vector: Vector::z_axis(),
            radius: 0.0,
            number_of_turns: 0.0,
            turn_height: 0.0,
            is_right_handed: false,
            constraint: HelixConstraint::ConstrainTurnHeight,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Image {
    pub class_version: i32,
    pub location: Point,
    pub u_vector: Vector,
    pub v_vector: Vector,
    pub image_size: Vector,
    pub image_def_reference: String,
    pub display_options_flags: i32,
    pub use_clipping: bool,
    pub brightness: i16,
    pub contrast: i16,
    pub fade: i16,
    pub image_def_reactor_reference: String,
    pub clipping_type: ImageClippingBoundaryType,
    pub clipping_vertex_count: i32,
    #[doc(hidden)]
    pub __clipping_vertices_x: Vec<f64>,
    #[doc(hidden)]
    pub __clipping_vertices_y: Vec<f64>,
    pub clipping_vertices: Vec<Point>,
    pub is_inside_clipping: bool,
}

impl Default for Image {
    fn default() -> Image {
        Image {
            class_version: 0,
            location: Point::origin(),
            u_vector: Vector::x_axis(),
            v_vector: Vector::y_axis(),
            image_size: Vector::zero(),
            image_def_reference: String::new(),
            display_options_flags: 0,
            use_clipping: true,
            brightness: 50,
            contrast: 50,
            fade: 0,
            image_def_reactor_reference: String::new(),
            clipping_type: ImageClippingBoundaryType::Rectangular,
            clipping_vertex_count: 0,
            __clipping_vertices_x: vec![],
            __clipping_vertices_y: vec![],
            clipping_vertices: vec![],
            is_inside_clipping: false,
        }
    }
}

impl Image {
    pub fn get_show_image(&self) -> bool {
        self.display_options_flags & 1 != 0
    }
    pub fn set_show_image(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 1;
        }
        else {
            self.display_options_flags &= !1;
        }
    }
    pub fn get_show_image_when_not_alligned(&self) -> bool {
        self.display_options_flags & 2 != 0
    }
    pub fn set_show_image_when_not_alligned(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 2;
        }
        else {
            self.display_options_flags &= !2;
        }
    }
    pub fn get_use_clipping_boundary(&self) -> bool {
        self.display_options_flags & 4 != 0
    }
    pub fn set_use_clipping_boundary(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 4;
        }
        else {
            self.display_options_flags &= !4;
        }
    }
    pub fn get_use_transparency(&self) -> bool {
        self.display_options_flags & 8 != 0
    }
    pub fn set_use_transparency(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 8;
        }
        else {
            self.display_options_flags &= !8;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Insert {
    #[doc(hidden)]
    pub __has_attributes: bool,
    pub name: String,
    pub location: Point,
    pub x_scale_factor: f64,
    pub y_scale_factor: f64,
    pub z_scale_factor: f64,
    pub rotation: f64,
    pub column_count: i16,
    pub row_count: i16,
    pub column_spacing: f64,
    pub row_spacing: f64,
    pub extrusion_direction: Vector,
    #[doc(hidden)]
    pub __attributes_and_handles: Vec<(Attribute, Handle)>,
}

impl Default for Insert {
    fn default() -> Insert {
        Insert {
            __has_attributes: false,
            name: String::new(),
            location: Point::origin(),
            x_scale_factor: 1.0,
            y_scale_factor: 1.0,
            z_scale_factor: 1.0,
            rotation: 0.0,
            column_count: 1,
            row_count: 1,
            column_spacing: 0.0,
            row_spacing: 0.0,
            extrusion_direction: Vector::z_axis(),
            __attributes_and_handles: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Leader {
    pub dimension_style_name: String,
    pub use_arrowheads: bool,
    pub path_type: LeaderPathType,
    pub annotation_type: LeaderCreationAnnotationType,
    pub hookline_direction: LeaderHooklineDirection,
    pub use_hookline: bool,
    pub text_annotation_height: f64,
    pub text_annotation_width: f64,
    pub vertex_count: i32,
    #[doc(hidden)]
    pub __vertices_x: Vec<f64>,
    #[doc(hidden)]
    pub __vertices_y: Vec<f64>,
    #[doc(hidden)]
    pub __vertices_z: Vec<f64>,
    pub vertices: Vec<Point>,
    pub override_color: Color,
    pub associated_annotation_reference: String,
    pub normal: Vector,
    pub right: Vector,
    pub block_offset: Vector,
    pub annotation_offset: Vector,
}

impl Default for Leader {
    fn default() -> Leader {
        Leader {
            dimension_style_name: String::new(),
            use_arrowheads: true,
            path_type: LeaderPathType::StraightLineSegments,
            annotation_type: LeaderCreationAnnotationType::NoAnnotation,
            hookline_direction: LeaderHooklineDirection::OppositeFromHorizontalVector,
            use_hookline: true,
            text_annotation_height: 1.0,
            text_annotation_width: 1.0,
            vertex_count: 0,
            __vertices_x: vec![],
            __vertices_y: vec![],
            __vertices_z: vec![],
            vertices: vec![],
            override_color: Color::by_block(),
            associated_annotation_reference: String::new(),
            normal: Vector::z_axis(),
            right: Vector::x_axis(),
            block_offset: Vector::zero(),
            annotation_offset: Vector::x_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Light {
    pub version_number: i32,
    pub name: String,
    pub light_type: LightType,
    pub is_active: bool,
    pub plot_glyph: bool,
    pub intensity: f64,
    pub position: Point,
    pub target_location: Point,
    pub attentuation_type: LightAttenuationType,
    pub use_attenuation_limits: bool,
    pub attenuation_start_limit: f64,
    pub attenuation_end_limit: f64,
    pub hotspot_angle: f64,
    pub falloff_angle: f64,
    pub cast_shadows: bool,
    pub shadow_type: ShadowType,
    pub shadow_map_size: i32,
    pub shadow_map_softness: i16,
}

impl Default for Light {
    fn default() -> Light {
        Light {
            version_number: 0,
            name: String::new(),
            light_type: LightType::Distant,
            is_active: true,
            plot_glyph: true,
            intensity: 1.0,
            position: Point::origin(),
            target_location: Point::origin(),
            attentuation_type: LightAttenuationType::None,
            use_attenuation_limits: true,
            attenuation_start_limit: 0.0,
            attenuation_end_limit: 1.0,
            hotspot_angle: 0.0,
            falloff_angle: 0.0,
            cast_shadows: true,
            shadow_type: ShadowType::RayTraced,
            shadow_map_size: 0,
            shadow_map_softness: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Line {
    pub thickness: f64,
    pub p1: Point,
    pub p2: Point,
    pub extrusion_direction: Vector,
}

impl Default for Line {
    fn default() -> Line {
        Line {
            thickness: 0.0,
            p1: Point::origin(),
            p2: Point::origin(),
            extrusion_direction: Vector::z_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct LwPolyline {
    pub flags: i32,
    pub constant_width: f64,
    pub thickness: f64,
    pub vertices: Vec<LwPolylineVertex>,
    pub extrusion_direction: Vector,
}

impl Default for LwPolyline {
    fn default() -> LwPolyline {
        LwPolyline {
            flags: 0,
            constant_width: 0.0,
            thickness: 0.0,
            vertices: vec![],
            extrusion_direction: Vector::z_axis(),
        }
    }
}

impl LwPolyline {
    pub fn get_is_closed(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_closed(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_pline_gen(&self) -> bool {
        self.flags & 128 != 0
    }
    pub fn set_is_pline_gen(&mut self, val: bool) {
        if val {
            self.flags |= 128;
        }
        else {
            self.flags &= !128;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct MLine {
    pub style_name: String,
    #[doc(hidden)]
    pub __style_handle: Handle,
    pub scale_factor: f64,
    pub justification: Justification,
    pub flags: i32,
    #[doc(hidden)]
    pub __vertex_count: i32,
    pub style_element_count: i32,
    pub start_point: Point,
    pub normal: Vector,
    #[doc(hidden)]
    pub __vertices_x: Vec<f64>,
    #[doc(hidden)]
    pub __vertices_y: Vec<f64>,
    #[doc(hidden)]
    pub __vertices_z: Vec<f64>,
    pub vertices: Vec<Point>,
    #[doc(hidden)]
    pub __segment_direction_x: Vec<f64>,
    #[doc(hidden)]
    pub __segment_direction_y: Vec<f64>,
    #[doc(hidden)]
    pub __segment_direction_z: Vec<f64>,
    pub segment_directions: Vec<Vector>,
    #[doc(hidden)]
    pub __miter_direction_x: Vec<f64>,
    #[doc(hidden)]
    pub __miter_direction_y: Vec<f64>,
    #[doc(hidden)]
    pub __miter_direction_z: Vec<f64>,
    pub miter_directions: Vec<Vector>,
    #[doc(hidden)]
    pub __parameter_count: i32,
    pub parameters: Vec<f64>,
    #[doc(hidden)]
    pub __area_fill_parameter_count: i32,
    pub area_fill_parameters: Vec<f64>,
}

impl Default for MLine {
    fn default() -> MLine {
        MLine {
            style_name: String::new(),
            __style_handle: Handle::empty(),
            scale_factor: 1.0,
            justification: Justification::Top,
            flags: 0,
            __vertex_count: 0,
            style_element_count: 0,
            start_point: Point::origin(),
            normal: Vector::z_axis(),
            __vertices_x: vec![],
            __vertices_y: vec![],
            __vertices_z: vec![],
            vertices: vec![],
            __segment_direction_x: vec![],
            __segment_direction_y: vec![],
            __segment_direction_z: vec![],
            segment_directions: vec![],
            __miter_direction_x: vec![],
            __miter_direction_y: vec![],
            __miter_direction_z: vec![],
            miter_directions: vec![],
            __parameter_count: 0,
            parameters: vec![],
            __area_fill_parameter_count: 0,
            area_fill_parameters: vec![],
        }
    }
}

impl MLine {
    pub fn get_has_at_least_one_vertex(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_has_at_least_one_vertex(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_closed(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_closed(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_suppress_start_caps(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_suppress_start_caps(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_suppress_end_caps(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_suppress_end_caps(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_style<'a>(&self, drawing: &'a Drawing) -> Option<&'a Style> {
        match drawing.get_item_by_handle(self.__style_handle) {
            Some(DrawingItem::Style(val)) => Some(val),
            _ => None,
        }
    }
    pub fn set_style(&mut self, item: &Style) {
        self.__style_handle = DrawingItem::Style(item).get_handle();
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct MText {
    pub insertion_point: Point,
    pub initial_text_height: f64,
    pub reference_rectangle_width: f64,
    pub attachment_point: AttachmentPoint,
    pub drawing_direction: DrawingDirection,
    pub extended_text: Vec<String>,
    pub text: String,
    pub text_style_name: String,
    pub extrusion_direction: Vector,
    pub x_axis_direction: Vector,
    pub horizontal_width: f64,
    pub vertical_height: f64,
    pub rotation_angle: f64,
    pub line_spacing_style: MTextLineSpacingStyle,
    pub line_spacing_factor: f64,
    pub background_fill_setting: BackgroundFillSetting,
    pub background_color_rgb: i32,
    pub background_color_name: String,
    pub fill_box_scale: f64,
    pub background_fill_color: Color,
    pub background_fill_color_transparency: i32,
    pub column_type: i16,
    pub column_count: i32,
    pub is_column_flow_reversed: bool,
    pub is_column_auto_height: bool,
    pub column_width: f64,
    pub column_gutter: f64,
    pub column_heights: Vec<f64>,
}

impl Default for MText {
    fn default() -> MText {
        MText {
            insertion_point: Point::origin(),
            initial_text_height: 1.0,
            reference_rectangle_width: 1.0,
            attachment_point: AttachmentPoint::TopLeft,
            drawing_direction: DrawingDirection::LeftToRight,
            extended_text: vec![],
            text: String::new(),
            text_style_name: String::from("STANDARD"),
            extrusion_direction: Vector::z_axis(),
            x_axis_direction: Vector::x_axis(),
            horizontal_width: 1.0,
            vertical_height: 1.0,
            rotation_angle: 0.0,
            line_spacing_style: MTextLineSpacingStyle::AtLeast,
            line_spacing_factor: 1.0,
            background_fill_setting: BackgroundFillSetting::Off,
            background_color_rgb: 0,
            background_color_name: String::new(),
            fill_box_scale: 1.0,
            background_fill_color: Color::by_layer(),
            background_fill_color_transparency: 0,
            column_type: 0,
            column_count: 0,
            is_column_flow_reversed: false,
            is_column_auto_height: true,
            column_width: 0.0,
            column_gutter: 0.0,
            column_heights: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct OleFrame {
    pub version_number: i32,
    pub binary_data_length: i32,
    pub binary_data_strings: Vec<Vec<u8>>,
}

impl Default for OleFrame {
    fn default() -> OleFrame {
        OleFrame {
            version_number: 0,
            binary_data_length: 0,
            binary_data_strings: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Ole2Frame {
    pub version_number: i32,
    pub description: String,
    pub upper_left_corner: Point,
    pub lower_right_corner: Point,
    pub object_type: OleObjectType,
    pub tile_mode: TileModeDescriptor,
    pub binary_data_length: i32,
    pub binary_data_strings: Vec<Vec<u8>>,
}

impl Default for Ole2Frame {
    fn default() -> Ole2Frame {
        Ole2Frame {
            version_number: 0,
            description: String::new(),
            upper_left_corner: Point::origin(),
            lower_right_corner: Point::origin(),
            object_type: OleObjectType::Static,
            tile_mode: TileModeDescriptor::InTiledViewport,
            binary_data_length: 0,
            binary_data_strings: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct ModelPoint {
    pub location: Point,
    pub thickness: f64,
    pub extrusion_direction: Vector,
    pub angle: f64,
}

impl Default for ModelPoint {
    fn default() -> ModelPoint {
        ModelPoint {
            location: Point::origin(),
            thickness: 0.0,
            extrusion_direction: Vector::z_axis(),
            angle: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Polyline {
    pub contains_vertices: bool,
    pub location: Point,
    pub thickness: f64,
    pub flags: i32,
    pub default_starting_width: f64,
    pub default_ending_width: f64,
    pub polygon_mesh_m_vertex_count: i32,
    pub polygon_mesh_n_vertex_count: i32,
    pub smooth_surface_m_density: i32,
    pub smooth_surface_n_density: i32,
    pub surface_type: PolylineCurvedAndSmoothSurfaceType,
    pub normal: Vector,
    #[doc(hidden)]
    pub __vertices_and_handles: Vec<(Vertex, Handle)>,
}

impl Default for Polyline {
    fn default() -> Polyline {
        Polyline {
            contains_vertices: true,
            location: Point::origin(),
            thickness: 0.0,
            flags: 0,
            default_starting_width: 0.0,
            default_ending_width: 0.0,
            polygon_mesh_m_vertex_count: 0,
            polygon_mesh_n_vertex_count: 0,
            smooth_surface_m_density: 0,
            smooth_surface_n_density: 0,
            surface_type: PolylineCurvedAndSmoothSurfaceType::None,
            normal: Vector::z_axis(),
            __vertices_and_handles: vec![],
        }
    }
}

impl Polyline {
    pub fn get_is_closed(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_closed(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_curve_fit_vertices_added(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_curve_fit_vertices_added(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_spline_fit_vertices_added(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_spline_fit_vertices_added(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_is_3d_polyline(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_is_3d_polyline(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_is_3d_polygon_mesh(&self) -> bool {
        self.flags & 16 != 0
    }
    pub fn set_is_3d_polygon_mesh(&mut self, val: bool) {
        if val {
            self.flags |= 16;
        }
        else {
            self.flags &= !16;
        }
    }
    pub fn get_is_polygon_mesh_closed_in_n_direction(&self) -> bool {
        self.flags & 32 != 0
    }
    pub fn set_is_polygon_mesh_closed_in_n_direction(&mut self, val: bool) {
        if val {
            self.flags |= 32;
        }
        else {
            self.flags &= !32;
        }
    }
    pub fn get_is_polyface_mesh(&self) -> bool {
        self.flags & 64 != 0
    }
    pub fn set_is_polyface_mesh(&mut self, val: bool) {
        if val {
            self.flags |= 64;
        }
        else {
            self.flags &= !64;
        }
    }
    pub fn get_is_line_type_pattern_generated_continuously(&self) -> bool {
        self.flags & 128 != 0
    }
    pub fn set_is_line_type_pattern_generated_continuously(&mut self, val: bool) {
        if val {
            self.flags |= 128;
        }
        else {
            self.flags &= !128;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Ray {
    pub start_point: Point,
    pub unit_direction_vector: Vector,
}

impl Default for Ray {
    fn default() -> Ray {
        Ray {
            start_point: Point::origin(),
            unit_direction_vector: Vector::x_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Region {
    pub format_version_number: i16,
    pub custom_data: Vec<String>,
    pub custom_data2: Vec<String>,
}

impl Default for Region {
    fn default() -> Region {
        Region {
            format_version_number: 1,
            custom_data: vec![],
            custom_data2: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct RText {
    pub insertion_point: Point,
    pub extrusion_direction: Vector,
    pub rotation_angle: f64,
    pub text_height: f64,
    pub text_style: String,
    pub type_flags: i32,
    pub contents: String,
}

impl Default for RText {
    fn default() -> RText {
        RText {
            insertion_point: Point::origin(),
            extrusion_direction: Vector::z_axis(),
            rotation_angle: 0.0,
            text_height: 0.0,
            text_style: String::from("STANDARD"),
            type_flags: 0,
            contents: String::new(),
        }
    }
}

impl RText {
    pub fn get_is_expression(&self) -> bool {
        self.type_flags & 1 != 0
    }
    pub fn set_is_expression(&mut self, val: bool) {
        if val {
            self.type_flags |= 1;
        }
        else {
            self.type_flags &= !1;
        }
    }
    pub fn get_is_inline_mtext_sequences_enabled(&self) -> bool {
        self.type_flags & 2 != 0
    }
    pub fn set_is_inline_mtext_sequences_enabled(&mut self, val: bool) {
        if val {
            self.type_flags |= 2;
        }
        else {
            self.type_flags &= !2;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Section {
    pub state: i32,
    pub flags: i32,
    pub name: String,
    pub vertical_direction: Vector,
    pub top_height: f64,
    pub bottom_height: f64,
    pub indicator_transparency: i16,
    pub indicator_color: Color,
    pub indicator_color_name: String,
    #[doc(hidden)]
    pub __vertex_count: i32,
    #[doc(hidden)]
    pub __vertices_x: Vec<f64>,
    #[doc(hidden)]
    pub __vertices_y: Vec<f64>,
    #[doc(hidden)]
    pub __vertices_z: Vec<f64>,
    pub vertices: Vec<Point>,
    #[doc(hidden)]
    pub __back_line_vertex_count: i32,
    #[doc(hidden)]
    pub __back_line_vertices_x: Vec<f64>,
    #[doc(hidden)]
    pub __back_line_vertices_y: Vec<f64>,
    #[doc(hidden)]
    pub __back_line_vertices_z: Vec<f64>,
    pub back_line_vertices: Vec<Point>,
    #[doc(hidden)]
    pub __geometry_settings_handle: Handle,
}

impl Default for Section {
    fn default() -> Section {
        Section {
            state: 0,
            flags: 0,
            name: String::new(),
            vertical_direction: Vector::z_axis(),
            top_height: 0.0,
            bottom_height: 0.0,
            indicator_transparency: 0,
            indicator_color: Color::by_layer(),
            indicator_color_name: String::new(),
            __vertex_count: 0,
            __vertices_x: vec![],
            __vertices_y: vec![],
            __vertices_z: vec![],
            vertices: vec![],
            __back_line_vertex_count: 0,
            __back_line_vertices_x: vec![],
            __back_line_vertices_y: vec![],
            __back_line_vertices_z: vec![],
            back_line_vertices: vec![],
            __geometry_settings_handle: Handle::empty(),
        }
    }
}

impl Section {
    pub fn get_geometry_settings<'a>(&self, drawing: &'a Drawing) -> Option<DrawingItem<'a>> {
        drawing.get_item_by_handle(self.__geometry_settings_handle)
    }
    pub fn set_geometry_settings(&mut self, item: &DrawingItemMut) {
        self.__geometry_settings_handle = item.get_handle();
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Seqend {
}

impl Default for Seqend {
    fn default() -> Seqend {
        Seqend {
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Shape {
    pub thickness: f64,
    pub location: Point,
    pub size: f64,
    pub name: String,
    pub rotation_angle: f64,
    pub relative_x_scale_factor: f64,
    pub oblique_angle: f64,
    pub extrusion_direction: Vector,
}

impl Default for Shape {
    fn default() -> Shape {
        Shape {
            thickness: 0.0,
            location: Point::origin(),
            size: 0.0,
            name: String::new(),
            rotation_angle: 0.0,
            relative_x_scale_factor: 1.0,
            oblique_angle: 0.0,
            extrusion_direction: Vector::z_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Solid {
    pub first_corner: Point,
    pub second_corner: Point,
    pub third_corner: Point,
    pub fourth_corner: Point,
    pub thickness: f64,
    pub extrusion_direction: Vector,
}

impl Default for Solid {
    fn default() -> Solid {
        Solid {
            first_corner: Point::origin(),
            second_corner: Point::origin(),
            third_corner: Point::origin(),
            fourth_corner: Point::origin(),
            thickness: 0.0,
            extrusion_direction: Vector::z_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Spline {
    pub normal: Vector,
    pub flags: i32,
    pub degree_of_curve: i32,
    #[doc(hidden)]
    pub __number_of_knots_ignored: i32,
    #[doc(hidden)]
    pub __number_of_control_points_ignored: i32,
    #[doc(hidden)]
    pub __number_of_fit_points_ignored: i32,
    pub knot_tolerance: f64,
    pub control_point_tolerance: f64,
    pub fit_tolerance: f64,
    pub start_tangent: Point,
    pub end_tangent: Point,
    pub knot_values: Vec<f64>,
    pub weight: f64,
    #[doc(hidden)]
    pub __control_point_x: Vec<f64>,
    #[doc(hidden)]
    pub __control_point_y: Vec<f64>,
    #[doc(hidden)]
    pub __control_point_z: Vec<f64>,
    pub control_points: Vec<Point>,
    #[doc(hidden)]
    pub __fit_point_x: Vec<f64>,
    #[doc(hidden)]
    pub __fit_point_y: Vec<f64>,
    #[doc(hidden)]
    pub __fit_point_z: Vec<f64>,
    pub fit_points: Vec<Point>,
}

impl Default for Spline {
    fn default() -> Spline {
        Spline {
            normal: Vector::z_axis(),
            flags: 0,
            degree_of_curve: 1,
            __number_of_knots_ignored: 0,
            __number_of_control_points_ignored: 0,
            __number_of_fit_points_ignored: 0,
            knot_tolerance: 0.000_000_1,
            control_point_tolerance: 0.000_000_1,
            fit_tolerance: 0.000_000_000_1,
            start_tangent: Point::origin(),
            end_tangent: Point::origin(),
            knot_values: vec![],
            weight: 1.0,
            __control_point_x: vec![],
            __control_point_y: vec![],
            __control_point_z: vec![],
            control_points: vec![],
            __fit_point_x: vec![],
            __fit_point_y: vec![],
            __fit_point_z: vec![],
            fit_points: vec![],
        }
    }
}

impl Spline {
    pub fn get_is_closed(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_closed(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_periodic(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_periodic(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_is_rational(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_is_rational(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_is_planar(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_is_planar(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_is_linear(&self) -> bool {
        self.flags & 16 != 0
    }
    pub fn set_is_linear(&mut self, val: bool) {
        if val {
            self.flags |= 16;
        }
        else {
            self.flags &= !16;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Text {
    pub thickness: f64,
    pub location: Point,
    pub text_height: f64,
    pub value: String,
    pub rotation: f64,
    pub relative_x_scale_factor: f64,
    pub oblique_angle: f64,
    pub text_style_name: String,
    pub text_generation_flags: i32,
    pub horizontal_text_justification: HorizontalTextJustification,
    pub second_alignment_point: Point,
    pub normal: Vector,
    pub vertical_text_justification: VerticalTextJustification,
}

impl Default for Text {
    fn default() -> Text {
        Text {
            thickness: 0.0,
            location: Point::origin(),
            text_height: 1.0,
            value: String::new(),
            rotation: 0.0,
            relative_x_scale_factor: 1.0,
            oblique_angle: 0.0,
            text_style_name: String::from("STANDARD"),
            text_generation_flags: 0,
            horizontal_text_justification: HorizontalTextJustification::Left,
            second_alignment_point: Point::origin(),
            normal: Vector::z_axis(),
            vertical_text_justification: VerticalTextJustification::Baseline,
        }
    }
}

impl Text {
    pub fn get_is_text_backwards(&self) -> bool {
        self.text_generation_flags & 2 != 0
    }
    pub fn set_is_text_backwards(&mut self, val: bool) {
        if val {
            self.text_generation_flags |= 2;
        }
        else {
            self.text_generation_flags &= !2;
        }
    }
    pub fn get_is_text_upside_down(&self) -> bool {
        self.text_generation_flags & 4 != 0
    }
    pub fn set_is_text_upside_down(&mut self, val: bool) {
        if val {
            self.text_generation_flags |= 4;
        }
        else {
            self.text_generation_flags &= !4;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Tolerance {
    pub dimension_style_name: String,
    pub insertion_point: Point,
    pub display_text: String,
    pub extrusion_direction: Vector,
    pub direction_vector: Vector,
}

impl Default for Tolerance {
    fn default() -> Tolerance {
        Tolerance {
            dimension_style_name: String::new(),
            insertion_point: Point::origin(),
            display_text: String::new(),
            extrusion_direction: Vector::z_axis(),
            direction_vector: Vector::x_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Trace {
    pub first_corner: Point,
    pub second_corner: Point,
    pub third_corner: Point,
    pub fourth_corner: Point,
    pub thickness: f64,
    pub extrusion_direction: Vector,
}

impl Default for Trace {
    fn default() -> Trace {
        Trace {
            first_corner: Point::origin(),
            second_corner: Point::origin(),
            third_corner: Point::origin(),
            fourth_corner: Point::origin(),
            thickness: 0.0,
            extrusion_direction: Vector::z_axis(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct DgnUnderlay {
    #[doc(hidden)]
    pub __object_handle: Handle,
    pub insertion_point: Point,
    pub x_scale: f64,
    pub y_scale: f64,
    pub z_scale: f64,
    pub rotation_angle: f64,
    pub normal: Vector,
    pub flags: i32,
    pub contrast: i16,
    pub fade: i16,
    #[doc(hidden)]
    pub __point_x: Vec<f64>,
    #[doc(hidden)]
    pub __point_y: Vec<f64>,
    pub points: Vec<Point>,
}

impl Default for DgnUnderlay {
    fn default() -> DgnUnderlay {
        DgnUnderlay {
            __object_handle: Handle::empty(),
            insertion_point: Point::origin(),
            x_scale: 1.0,
            y_scale: 1.0,
            z_scale: 1.0,
            rotation_angle: 0.0,
            normal: Vector::z_axis(),
            flags: 0,
            contrast: 100,
            fade: 0,
            __point_x: vec![],
            __point_y: vec![],
            points: vec![],
        }
    }
}

impl DgnUnderlay {
    pub fn get_is_clipping_on(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_clipping_on(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_underlay_on(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_underlay_on(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_is_monochrome(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_is_monochrome(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_adjust_for_background(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_adjust_for_background(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_is_clip_inside_mode(&self) -> bool {
        self.flags & 16 != 0
    }
    pub fn set_is_clip_inside_mode(&mut self, val: bool) {
        if val {
            self.flags |= 16;
        }
        else {
            self.flags &= !16;
        }
    }
    pub fn get_object<'a>(&self, drawing: &'a Drawing) -> Option<&'a Object> {
        match drawing.get_item_by_handle(self.__object_handle) {
            Some(DrawingItem::Object(val)) => Some(val),
            _ => None,
        }
    }
    pub fn set_object(&mut self, item: &Object) {
        self.__object_handle = DrawingItem::Object(item).get_handle();
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct DwfUnderlay {
    #[doc(hidden)]
    pub __object_handle: Handle,
    pub insertion_point: Point,
    pub x_scale: f64,
    pub y_scale: f64,
    pub z_scale: f64,
    pub rotation_angle: f64,
    pub normal: Vector,
    pub flags: i32,
    pub contrast: i16,
    pub fade: i16,
    #[doc(hidden)]
    pub __point_x: Vec<f64>,
    #[doc(hidden)]
    pub __point_y: Vec<f64>,
    pub points: Vec<Point>,
}

impl Default for DwfUnderlay {
    fn default() -> DwfUnderlay {
        DwfUnderlay {
            __object_handle: Handle::empty(),
            insertion_point: Point::origin(),
            x_scale: 1.0,
            y_scale: 1.0,
            z_scale: 1.0,
            rotation_angle: 0.0,
            normal: Vector::z_axis(),
            flags: 0,
            contrast: 100,
            fade: 0,
            __point_x: vec![],
            __point_y: vec![],
            points: vec![],
        }
    }
}

impl DwfUnderlay {
    pub fn get_is_clipping_on(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_clipping_on(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_underlay_on(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_underlay_on(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_is_monochrome(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_is_monochrome(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_adjust_for_background(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_adjust_for_background(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_is_clip_inside_mode(&self) -> bool {
        self.flags & 16 != 0
    }
    pub fn set_is_clip_inside_mode(&mut self, val: bool) {
        if val {
            self.flags |= 16;
        }
        else {
            self.flags &= !16;
        }
    }
    pub fn get_object<'a>(&self, drawing: &'a Drawing) -> Option<&'a Object> {
        match drawing.get_item_by_handle(self.__object_handle) {
            Some(DrawingItem::Object(val)) => Some(val),
            _ => None,
        }
    }
    pub fn set_object(&mut self, item: &Object) {
        self.__object_handle = DrawingItem::Object(item).get_handle();
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct PdfUnderlay {
    #[doc(hidden)]
    pub __object_handle: Handle,
    pub insertion_point: Point,
    pub x_scale: f64,
    pub y_scale: f64,
    pub z_scale: f64,
    pub rotation_angle: f64,
    pub normal: Vector,
    pub flags: i32,
    pub contrast: i16,
    pub fade: i16,
    #[doc(hidden)]
    pub __point_x: Vec<f64>,
    #[doc(hidden)]
    pub __point_y: Vec<f64>,
    pub points: Vec<Point>,
}

impl Default for PdfUnderlay {
    fn default() -> PdfUnderlay {
        PdfUnderlay {
            __object_handle: Handle::empty(),
            insertion_point: Point::origin(),
            x_scale: 1.0,
            y_scale: 1.0,
            z_scale: 1.0,
            rotation_angle: 0.0,
            normal: Vector::z_axis(),
            flags: 0,
            contrast: 100,
            fade: 0,
            __point_x: vec![],
            __point_y: vec![],
            points: vec![],
        }
    }
}

impl PdfUnderlay {
    pub fn get_is_clipping_on(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_clipping_on(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_underlay_on(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_underlay_on(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_is_monochrome(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn set_is_monochrome(&mut self, val: bool) {
        if val {
            self.flags |= 4;
        }
        else {
            self.flags &= !4;
        }
    }
    pub fn get_adjust_for_background(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_adjust_for_background(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_is_clip_inside_mode(&self) -> bool {
        self.flags & 16 != 0
    }
    pub fn set_is_clip_inside_mode(&mut self, val: bool) {
        if val {
            self.flags |= 16;
        }
        else {
            self.flags &= !16;
        }
    }
    pub fn get_object<'a>(&self, drawing: &'a Drawing) -> Option<&'a Object> {
        match drawing.get_item_by_handle(self.__object_handle) {
            Some(DrawingItem::Object(val)) => Some(val),
            _ => None,
        }
    }
    pub fn set_object(&mut self, item: &Object) {
        self.__object_handle = DrawingItem::Object(item).get_handle();
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Vertex {
    pub location: Point,
    pub starting_width: f64,
    pub ending_width: f64,
    pub bulge: f64,
    pub flags: i32,
    pub curve_fit_tangent_direction: f64,
    pub polyface_mesh_vertex_index1: i32,
    pub polyface_mesh_vertex_index2: i32,
    pub polyface_mesh_vertex_index3: i32,
    pub polyface_mesh_vertex_index4: i32,
    pub identifier: i32,
}

impl Default for Vertex {
    fn default() -> Vertex {
        Vertex {
            location: Point::origin(),
            starting_width: 0.0,
            ending_width: 0.0,
            bulge: 0.0,
            flags: 0,
            curve_fit_tangent_direction: 0.0,
            polyface_mesh_vertex_index1: 0,
            polyface_mesh_vertex_index2: 0,
            polyface_mesh_vertex_index3: 0,
            polyface_mesh_vertex_index4: 0,
            identifier: 0,
        }
    }
}

impl Vertex {
    pub fn get_is_extra_created_by_curve_fit(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn set_is_extra_created_by_curve_fit(&mut self, val: bool) {
        if val {
            self.flags |= 1;
        }
        else {
            self.flags &= !1;
        }
    }
    pub fn get_is_curve_fit_tangent_defined(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn set_is_curve_fit_tangent_defined(&mut self, val: bool) {
        if val {
            self.flags |= 2;
        }
        else {
            self.flags &= !2;
        }
    }
    pub fn get_is_spline_vertex_created_by_spline_fitting(&self) -> bool {
        self.flags & 8 != 0
    }
    pub fn set_is_spline_vertex_created_by_spline_fitting(&mut self, val: bool) {
        if val {
            self.flags |= 8;
        }
        else {
            self.flags &= !8;
        }
    }
    pub fn get_is_spline_frame_control_point(&self) -> bool {
        self.flags & 16 != 0
    }
    pub fn set_is_spline_frame_control_point(&mut self, val: bool) {
        if val {
            self.flags |= 16;
        }
        else {
            self.flags &= !16;
        }
    }
    pub fn get_is_3d_polyline_vertex(&self) -> bool {
        self.flags & 32 != 0
    }
    pub fn set_is_3d_polyline_vertex(&mut self, val: bool) {
        if val {
            self.flags |= 32;
        }
        else {
            self.flags &= !32;
        }
    }
    pub fn get_is_3d_polygon_mesh(&self) -> bool {
        self.flags & 64 != 0
    }
    pub fn set_is_3d_polygon_mesh(&mut self, val: bool) {
        if val {
            self.flags |= 64;
        }
        else {
            self.flags &= !64;
        }
    }
    pub fn get_is_polyface_mesh_vertex(&self) -> bool {
        self.flags & 128 != 0
    }
    pub fn set_is_polyface_mesh_vertex(&mut self, val: bool) {
        if val {
            self.flags |= 128;
        }
        else {
            self.flags &= !128;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct Wipeout {
    pub class_version: i32,
    pub location: Point,
    pub u_vector: Vector,
    pub v_vector: Vector,
    pub image_size: Vector,
    pub image_def_reference: String,
    pub display_options_flags: i32,
    pub use_clipping: bool,
    pub brightness: i16,
    pub contrast: i16,
    pub fade: i16,
    pub image_def_reactor_reference: String,
    pub clipping_type: ImageClippingBoundaryType,
    pub clipping_vertex_count: i32,
    #[doc(hidden)]
    pub __clipping_vertices_x: Vec<f64>,
    #[doc(hidden)]
    pub __clipping_vertices_y: Vec<f64>,
    pub clipping_vertices: Vec<Point>,
    pub is_inside_clipping: bool,
}

impl Default for Wipeout {
    fn default() -> Wipeout {
        Wipeout {
            class_version: 0,
            location: Point::origin(),
            u_vector: Vector::x_axis(),
            v_vector: Vector::y_axis(),
            image_size: Vector::zero(),
            image_def_reference: String::new(),
            display_options_flags: 0,
            use_clipping: true,
            brightness: 50,
            contrast: 50,
            fade: 0,
            image_def_reactor_reference: String::new(),
            clipping_type: ImageClippingBoundaryType::Rectangular,
            clipping_vertex_count: 0,
            __clipping_vertices_x: vec![],
            __clipping_vertices_y: vec![],
            clipping_vertices: vec![],
            is_inside_clipping: false,
        }
    }
}

impl Wipeout {
    pub fn get_show_image(&self) -> bool {
        self.display_options_flags & 1 != 0
    }
    pub fn set_show_image(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 1;
        }
        else {
            self.display_options_flags &= !1;
        }
    }
    pub fn get_show_image_when_not_alligned(&self) -> bool {
        self.display_options_flags & 2 != 0
    }
    pub fn set_show_image_when_not_alligned(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 2;
        }
        else {
            self.display_options_flags &= !2;
        }
    }
    pub fn get_use_clipping_boundary(&self) -> bool {
        self.display_options_flags & 4 != 0
    }
    pub fn set_use_clipping_boundary(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 4;
        }
        else {
            self.display_options_flags &= !4;
        }
    }
    pub fn get_use_transparency(&self) -> bool {
        self.display_options_flags & 8 != 0
    }
    pub fn set_use_transparency(&mut self, val: bool) {
        if val {
            self.display_options_flags |= 8;
        }
        else {
            self.display_options_flags &= !8;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct XLine {
    pub first_point: Point,
    pub unit_direction_vector: Vector,
}

impl Default for XLine {
    fn default() -> XLine {
        XLine {
            first_point: Point::origin(),
            unit_direction_vector: Vector::x_axis(),
        }
    }
}

impl EntityType {
    pub(crate) fn is_supported_on_version(&self, version: AcadVersion) -> bool {
        match self {
            &EntityType::Face3D(_) => { true },
            &EntityType::Solid3D(_) => { version >= AcadVersion::R13 },
            &EntityType::ProxyEntity(_) => { version >= AcadVersion::R14 },
            &EntityType::Arc(_) => { true },
            &EntityType::ArcAlignedText(_) => { version >= AcadVersion::R2000 && version <= AcadVersion::R2000 },
            &EntityType::AttributeDefinition(_) => { true },
            &EntityType::Attribute(_) => { true },
            &EntityType::Body(_) => { version >= AcadVersion::R13 },
            &EntityType::Circle(_) => { true },
            &EntityType::RotatedDimension(_) => { true },
            &EntityType::RadialDimension(_) => { true },
            &EntityType::DiameterDimension(_) => { true },
            &EntityType::AngularThreePointDimension(_) => { true },
            &EntityType::OrdinateDimension(_) => { true },
            &EntityType::Ellipse(_) => { version >= AcadVersion::R13 },
            &EntityType::Helix(_) => { version >= AcadVersion::R2007 },
            &EntityType::Image(_) => { version >= AcadVersion::R14 },
            &EntityType::Insert(_) => { true },
            &EntityType::Leader(_) => { version >= AcadVersion::R13 },
            &EntityType::Light(_) => { version >= AcadVersion::R2007 },
            &EntityType::Line(_) => { true },
            &EntityType::LwPolyline(_) => { version >= AcadVersion::R14 },
            &EntityType::MLine(_) => { version >= AcadVersion::R13 },
            &EntityType::MText(_) => { version >= AcadVersion::R13 },
            &EntityType::OleFrame(_) => { version >= AcadVersion::R13 },
            &EntityType::Ole2Frame(_) => { version >= AcadVersion::R14 },
            &EntityType::ModelPoint(_) => { true },
            &EntityType::Polyline(_) => { true },
            &EntityType::Ray(_) => { version >= AcadVersion::R13 },
            &EntityType::Region(_) => { version >= AcadVersion::R13 },
            &EntityType::RText(_) => { version >= AcadVersion::R2000 && version <= AcadVersion::R2000 },
            &EntityType::Section(_) => { version >= AcadVersion::R2007 },
            &EntityType::Seqend(_) => { true },
            &EntityType::Shape(_) => { true },
            &EntityType::Solid(_) => { true },
            &EntityType::Spline(_) => { version >= AcadVersion::R13 },
            &EntityType::Text(_) => { true },
            &EntityType::Tolerance(_) => { version >= AcadVersion::R13 },
            &EntityType::Trace(_) => { true },
            &EntityType::DgnUnderlay(_) => { version >= AcadVersion::R2007 },
            &EntityType::DwfUnderlay(_) => { version >= AcadVersion::R2007 },
            &EntityType::PdfUnderlay(_) => { version >= AcadVersion::R2007 },
            &EntityType::Vertex(_) => { true },
            &EntityType::Wipeout(_) => { version >= AcadVersion::R2000 },
            &EntityType::XLine(_) => { version >= AcadVersion::R13 },
        }
    }
    pub(crate) fn from_type_string(type_string: &str) -> Option<EntityType> {
        match type_string {
            "3DFACE" => Some(EntityType::Face3D(Default::default())),
            "3DSOLID" => Some(EntityType::Solid3D(Default::default())),
            "ACAD_PROXY_ENTITY" => Some(EntityType::ProxyEntity(Default::default())),
            "ARC" => Some(EntityType::Arc(Default::default())),
            "ARCALIGNEDTEXT" => Some(EntityType::ArcAlignedText(Default::default())),
            "ATTDEF" => Some(EntityType::AttributeDefinition(Default::default())),
            "ATTRIB" => Some(EntityType::Attribute(Default::default())),
            "BODY" => Some(EntityType::Body(Default::default())),
            "CIRCLE" => Some(EntityType::Circle(Default::default())),
            "ELLIPSE" => Some(EntityType::Ellipse(Default::default())),
            "HELIX" => Some(EntityType::Helix(Default::default())),
            "IMAGE" => Some(EntityType::Image(Default::default())),
            "INSERT" => Some(EntityType::Insert(Default::default())),
            "LEADER" => Some(EntityType::Leader(Default::default())),
            "LIGHT" => Some(EntityType::Light(Default::default())),
            "LINE" => Some(EntityType::Line(Default::default())),
            "3DLINE" => Some(EntityType::Line(Default::default())),
            "LWPOLYLINE" => Some(EntityType::LwPolyline(Default::default())),
            "MLINE" => Some(EntityType::MLine(Default::default())),
            "MTEXT" => Some(EntityType::MText(Default::default())),
            "OLEFRAME" => Some(EntityType::OleFrame(Default::default())),
            "OLE2FRAME" => Some(EntityType::Ole2Frame(Default::default())),
            "POINT" => Some(EntityType::ModelPoint(Default::default())),
            "POLYLINE" => Some(EntityType::Polyline(Default::default())),
            "RAY" => Some(EntityType::Ray(Default::default())),
            "REGION" => Some(EntityType::Region(Default::default())),
            "RTEXT" => Some(EntityType::RText(Default::default())),
            "SECTION" => Some(EntityType::Section(Default::default())),
            "SEQEND" => Some(EntityType::Seqend(Default::default())),
            "SHAPE" => Some(EntityType::Shape(Default::default())),
            "SOLID" => Some(EntityType::Solid(Default::default())),
            "SPLINE" => Some(EntityType::Spline(Default::default())),
            "TEXT" => Some(EntityType::Text(Default::default())),
            "TOLERANCE" => Some(EntityType::Tolerance(Default::default())),
            "TRACE" => Some(EntityType::Trace(Default::default())),
            "DGNUNDERLAY" => Some(EntityType::DgnUnderlay(Default::default())),
            "DWFUNDERLAY" => Some(EntityType::DwfUnderlay(Default::default())),
            "PDFUNDERLAY" => Some(EntityType::PdfUnderlay(Default::default())),
            "VERTEX" => Some(EntityType::Vertex(Default::default())),
            "WIPEOUT" => Some(EntityType::Wipeout(Default::default())),
            "XLINE" => Some(EntityType::XLine(Default::default())),
            _ => None,
        }
    }
    pub(crate) fn to_type_string(&self) -> &str {
        match self {
            &EntityType::Face3D(_) => { "3DFACE" },
            &EntityType::Solid3D(_) => { "3DSOLID" },
            &EntityType::ProxyEntity(_) => { "ACAD_PROXY_ENTITY" },
            &EntityType::Arc(_) => { "ARC" },
            &EntityType::ArcAlignedText(_) => { "ARCALIGNEDTEXT" },
            &EntityType::AttributeDefinition(_) => { "ATTDEF" },
            &EntityType::Attribute(_) => { "ATTRIB" },
            &EntityType::Body(_) => { "BODY" },
            &EntityType::Circle(_) => { "CIRCLE" },
            &EntityType::RotatedDimension(_) => { "DIMENSION" },
            &EntityType::RadialDimension(_) => { "DIMENSION" },
            &EntityType::DiameterDimension(_) => { "DIMENSION" },
            &EntityType::AngularThreePointDimension(_) => { "DIMENSION" },
            &EntityType::OrdinateDimension(_) => { "DIMENSION" },
            &EntityType::Ellipse(_) => { "ELLIPSE" },
            &EntityType::Helix(_) => { "HELIX" },
            &EntityType::Image(_) => { "IMAGE" },
            &EntityType::Insert(_) => { "INSERT" },
            &EntityType::Leader(_) => { "LEADER" },
            &EntityType::Light(_) => { "LIGHT" },
            &EntityType::Line(_) => { "LINE" },
            &EntityType::LwPolyline(_) => { "LWPOLYLINE" },
            &EntityType::MLine(_) => { "MLINE" },
            &EntityType::MText(_) => { "MTEXT" },
            &EntityType::OleFrame(_) => { "OLEFRAME" },
            &EntityType::Ole2Frame(_) => { "OLE2FRAME" },
            &EntityType::ModelPoint(_) => { "POINT" },
            &EntityType::Polyline(_) => { "POLYLINE" },
            &EntityType::Ray(_) => { "RAY" },
            &EntityType::Region(_) => { "REGION" },
            &EntityType::RText(_) => { "RTEXT" },
            &EntityType::Section(_) => { "SECTION" },
            &EntityType::Seqend(_) => { "SEQEND" },
            &EntityType::Shape(_) => { "SHAPE" },
            &EntityType::Solid(_) => { "SOLID" },
            &EntityType::Spline(_) => { "SPLINE" },
            &EntityType::Text(_) => { "TEXT" },
            &EntityType::Tolerance(_) => { "TOLERANCE" },
            &EntityType::Trace(_) => { "TRACE" },
            &EntityType::DgnUnderlay(_) => { "DGNUNDERLAY" },
            &EntityType::DwfUnderlay(_) => { "DWFUNDERLAY" },
            &EntityType::PdfUnderlay(_) => { "PDFUNDERLAY" },
            &EntityType::Vertex(_) => { "VERTEX" },
            &EntityType::Wipeout(_) => { "WIPEOUT" },
            &EntityType::XLine(_) => { "XLINE" },
        }
    }
    pub(crate) fn try_apply_code_pair(&mut self, pair: &CodePair) -> DxfResult<bool> {
        match self {
            &mut EntityType::Face3D(ref mut ent) => {
                match pair.code {
                    10 => { ent.first_corner.x = pair.assert_f64()?; },
                    20 => { ent.first_corner.y = pair.assert_f64()?; },
                    30 => { ent.first_corner.z = pair.assert_f64()?; },
                    11 => { ent.second_corner.x = pair.assert_f64()?; },
                    21 => { ent.second_corner.y = pair.assert_f64()?; },
                    31 => { ent.second_corner.z = pair.assert_f64()?; },
                    12 => { ent.third_corner.x = pair.assert_f64()?; },
                    22 => { ent.third_corner.y = pair.assert_f64()?; },
                    32 => { ent.third_corner.z = pair.assert_f64()?; },
                    13 => { ent.fourth_corner.x = pair.assert_f64()?; },
                    23 => { ent.fourth_corner.y = pair.assert_f64()?; },
                    33 => { ent.fourth_corner.z = pair.assert_f64()?; },
                    70 => { ent.edge_flags = pair.assert_i16()? as i32; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Solid3D(ref mut ent) => {
                match pair.code {
                    70 => { ent.format_version_number = pair.assert_i16()?; },
                    1 => { ent.custom_data.push(pair.assert_string()?); },
                    3 => { ent.custom_data2.push(pair.assert_string()?); },
                    350 => { ent.__history_object_handle = pair.as_handle()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::ProxyEntity(ref mut ent) => {
                match pair.code {
                    90 => { ent.proxy_entity_class_id = pair.assert_i32()?; },
                    91 => { ent.application_entity_class_id = pair.assert_i32()?; },
                    92 => { ent.graphics_data_size = pair.assert_i32()?; },
                    310 => { ent.graphics_data_string.push(pair.assert_binary()?); },
                    93 => { ent.entity_data_size = pair.assert_i32()?; },
                    330 => { ent.object_id_1.push(pair.assert_string()?); },
                    340 => { ent.object_id_2.push(pair.assert_string()?); },
                    350 => { ent.object_id_3.push(pair.assert_string()?); },
                    360 => { ent.object_id_4.push(pair.assert_string()?); },
                    94 => { ent.terminator = pair.assert_i32()?; },
                    95 => { ent.__object_drawing_format = pair.assert_i32()? as u32; },
                    70 => { ent.original_data_format_is_dxf = as_bool(pair.assert_i16()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Arc(ref mut ent) => {
                match pair.code {
                    39 => { ent.thickness = pair.assert_f64()?; },
                    10 => { ent.center.x = pair.assert_f64()?; },
                    20 => { ent.center.y = pair.assert_f64()?; },
                    30 => { ent.center.z = pair.assert_f64()?; },
                    40 => { ent.radius = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    50 => { ent.start_angle = pair.assert_f64()?; },
                    51 => { ent.end_angle = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::ArcAlignedText(ref mut ent) => {
                match pair.code {
                    1 => { ent.text = pair.assert_string()?; },
                    2 => { ent.font_name = pair.assert_string()?; },
                    3 => { ent.bigfont_name = pair.assert_string()?; },
                    7 => { ent.text_style_name = pair.assert_string()?; },
                    10 => { ent.center_point.x = pair.assert_f64()?; },
                    20 => { ent.center_point.y = pair.assert_f64()?; },
                    30 => { ent.center_point.z = pair.assert_f64()?; },
                    40 => { ent.arc_radius = pair.assert_f64()?; },
                    41 => { ent.width_factor = pair.assert_f64()?; },
                    42 => { ent.text_height = pair.assert_f64()?; },
                    43 => { ent.character_spacing = pair.assert_f64()?; },
                    44 => { ent.offset_from_arc = pair.assert_f64()?; },
                    45 => { ent.right_offset = pair.assert_f64()?; },
                    46 => { ent.left_offset = pair.assert_f64()?; },
                    50 => { ent.start_angle = pair.assert_f64()?; },
                    51 => { ent.end_angle = pair.assert_f64()?; },
                    70 => { ent.is_character_order_reversed = as_bool(pair.assert_i16()?); },
                    71 => { ent.direction_flag = pair.assert_i16()?; },
                    72 => { ent.alignment_flag = pair.assert_i16()?; },
                    73 => { ent.side_flag = pair.assert_i16()?; },
                    74 => { ent.is_bold = as_bool(pair.assert_i16()?); },
                    75 => { ent.is_italic = as_bool(pair.assert_i16()?); },
                    76 => { ent.is_underline = as_bool(pair.assert_i16()?); },
                    77 => { ent.character_set_value = pair.assert_i16()?; },
                    78 => { ent.pitch_and_family_value = pair.assert_i16()?; },
                    79 => { ent.font_type = enum_from_number!(FontType, TTF, from_i16, pair.assert_i16()?); },
                    90 => { ent.color_index = pair.assert_i32()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    280 => { ent.wizard_flag = pair.assert_i16()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::AttributeDefinition(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::Attribute(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::Body(ref mut ent) => {
                match pair.code {
                    70 => { ent.format_version_number = pair.assert_i16()?; },
                    1 => { ent.custom_data.push(pair.assert_string()?); },
                    3 => { ent.custom_data2.push(pair.assert_string()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Circle(ref mut ent) => {
                match pair.code {
                    39 => { ent.thickness = pair.assert_f64()?; },
                    10 => { ent.center.x = pair.assert_f64()?; },
                    20 => { ent.center.y = pair.assert_f64()?; },
                    30 => { ent.center.z = pair.assert_f64()?; },
                    40 => { ent.radius = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::RotatedDimension(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::RadialDimension(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::DiameterDimension(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::AngularThreePointDimension(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::OrdinateDimension(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::Ellipse(ref mut ent) => {
                match pair.code {
                    10 => { ent.center.x = pair.assert_f64()?; },
                    20 => { ent.center.y = pair.assert_f64()?; },
                    30 => { ent.center.z = pair.assert_f64()?; },
                    11 => { ent.major_axis.x = pair.assert_f64()?; },
                    21 => { ent.major_axis.y = pair.assert_f64()?; },
                    31 => { ent.major_axis.z = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    40 => { ent.minor_axis_ratio = pair.assert_f64()?; },
                    41 => { ent.start_parameter = pair.assert_f64()?; },
                    42 => { ent.end_parameter = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Helix(ref mut ent) => {
                match pair.code {
                    90 => { ent.major_release_number = pair.assert_i32()?; },
                    91 => { ent.maintainence_release_number = pair.assert_i32()?; },
                    10 => { ent.axis_base_point.x = pair.assert_f64()?; },
                    20 => { ent.axis_base_point.y = pair.assert_f64()?; },
                    30 => { ent.axis_base_point.z = pair.assert_f64()?; },
                    11 => { ent.start_point.x = pair.assert_f64()?; },
                    21 => { ent.start_point.y = pair.assert_f64()?; },
                    31 => { ent.start_point.z = pair.assert_f64()?; },
                    12 => { ent.axis_vector.x = pair.assert_f64()?; },
                    22 => { ent.axis_vector.y = pair.assert_f64()?; },
                    32 => { ent.axis_vector.z = pair.assert_f64()?; },
                    40 => { ent.radius = pair.assert_f64()?; },
                    41 => { ent.number_of_turns = pair.assert_f64()?; },
                    42 => { ent.turn_height = pair.assert_f64()?; },
                    290 => { ent.is_right_handed = pair.assert_bool()?; },
                    280 => { ent.constraint = enum_from_number!(HelixConstraint, ConstrainTurnHeight, from_i16, pair.assert_i16()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Image(ref mut ent) => {
                match pair.code {
                    90 => { ent.class_version = pair.assert_i32()?; },
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    11 => { ent.u_vector.x = pair.assert_f64()?; },
                    21 => { ent.u_vector.y = pair.assert_f64()?; },
                    31 => { ent.u_vector.z = pair.assert_f64()?; },
                    12 => { ent.v_vector.x = pair.assert_f64()?; },
                    22 => { ent.v_vector.y = pair.assert_f64()?; },
                    32 => { ent.v_vector.z = pair.assert_f64()?; },
                    13 => { ent.image_size.x = pair.assert_f64()?; },
                    23 => { ent.image_size.y = pair.assert_f64()?; },
                    340 => { ent.image_def_reference = pair.assert_string()?; },
                    70 => { ent.display_options_flags = pair.assert_i16()? as i32; },
                    280 => { ent.use_clipping = as_bool(pair.assert_i16()?); },
                    281 => { ent.brightness = pair.assert_i16()?; },
                    282 => { ent.contrast = pair.assert_i16()?; },
                    283 => { ent.fade = pair.assert_i16()?; },
                    360 => { ent.image_def_reactor_reference = pair.assert_string()?; },
                    71 => { ent.clipping_type = enum_from_number!(ImageClippingBoundaryType, Rectangular, from_i16, pair.assert_i16()?); },
                    91 => { ent.clipping_vertex_count = pair.assert_i32()?; },
                    14 => { ent.__clipping_vertices_x.push(pair.assert_f64()?); },
                    24 => { ent.__clipping_vertices_y.push(pair.assert_f64()?); },
                    290 => { ent.is_inside_clipping = pair.assert_bool()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Insert(ref mut ent) => {
                match pair.code {
                    66 => { ent.__has_attributes = as_bool(pair.assert_i16()?); },
                    2 => { ent.name = pair.assert_string()?; },
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    41 => { ent.x_scale_factor = pair.assert_f64()?; },
                    42 => { ent.y_scale_factor = pair.assert_f64()?; },
                    43 => { ent.z_scale_factor = pair.assert_f64()?; },
                    50 => { ent.rotation = pair.assert_f64()?; },
                    70 => { ent.column_count = pair.assert_i16()?; },
                    71 => { ent.row_count = pair.assert_i16()?; },
                    44 => { ent.column_spacing = pair.assert_f64()?; },
                    45 => { ent.row_spacing = pair.assert_f64()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Leader(ref mut ent) => {
                match pair.code {
                    3 => { ent.dimension_style_name = pair.assert_string()?; },
                    71 => { ent.use_arrowheads = as_bool(pair.assert_i16()?); },
                    72 => { ent.path_type = enum_from_number!(LeaderPathType, StraightLineSegments, from_i16, pair.assert_i16()?); },
                    73 => { ent.annotation_type = enum_from_number!(LeaderCreationAnnotationType, NoAnnotation, from_i16, pair.assert_i16()?); },
                    74 => { ent.hookline_direction = enum_from_number!(LeaderHooklineDirection, OppositeFromHorizontalVector, from_i16, pair.assert_i16()?); },
                    75 => { ent.use_hookline = as_bool(pair.assert_i16()?); },
                    40 => { ent.text_annotation_height = pair.assert_f64()?; },
                    41 => { ent.text_annotation_width = pair.assert_f64()?; },
                    76 => { ent.vertex_count = pair.assert_i16()? as i32; },
                    10 => { ent.__vertices_x.push(pair.assert_f64()?); },
                    20 => { ent.__vertices_y.push(pair.assert_f64()?); },
                    30 => { ent.__vertices_z.push(pair.assert_f64()?); },
                    77 => { ent.override_color = Color::from_raw_value(pair.assert_i16()?); },
                    340 => { ent.associated_annotation_reference = pair.assert_string()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    211 => { ent.right.x = pair.assert_f64()?; },
                    221 => { ent.right.y = pair.assert_f64()?; },
                    231 => { ent.right.z = pair.assert_f64()?; },
                    212 => { ent.block_offset.x = pair.assert_f64()?; },
                    222 => { ent.block_offset.y = pair.assert_f64()?; },
                    232 => { ent.block_offset.z = pair.assert_f64()?; },
                    213 => { ent.annotation_offset.x = pair.assert_f64()?; },
                    223 => { ent.annotation_offset.y = pair.assert_f64()?; },
                    233 => { ent.annotation_offset.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Light(ref mut ent) => {
                match pair.code {
                    90 => { ent.version_number = pair.assert_i32()?; },
                    1 => { ent.name = pair.assert_string()?; },
                    70 => { ent.light_type = enum_from_number!(LightType, Distant, from_i16, pair.assert_i16()?); },
                    290 => { ent.is_active = pair.assert_bool()?; },
                    291 => { ent.plot_glyph = pair.assert_bool()?; },
                    40 => { ent.intensity = pair.assert_f64()?; },
                    10 => { ent.position.x = pair.assert_f64()?; },
                    20 => { ent.position.y = pair.assert_f64()?; },
                    30 => { ent.position.z = pair.assert_f64()?; },
                    11 => { ent.target_location.x = pair.assert_f64()?; },
                    21 => { ent.target_location.y = pair.assert_f64()?; },
                    31 => { ent.target_location.z = pair.assert_f64()?; },
                    72 => { ent.attentuation_type = enum_from_number!(LightAttenuationType, None, from_i16, pair.assert_i16()?); },
                    292 => { ent.use_attenuation_limits = pair.assert_bool()?; },
                    41 => { ent.attenuation_start_limit = pair.assert_f64()?; },
                    42 => { ent.attenuation_end_limit = pair.assert_f64()?; },
                    50 => { ent.hotspot_angle = pair.assert_f64()?; },
                    51 => { ent.falloff_angle = pair.assert_f64()?; },
                    293 => { ent.cast_shadows = pair.assert_bool()?; },
                    73 => { ent.shadow_type = enum_from_number!(ShadowType, RayTraced, from_i16, pair.assert_i16()?); },
                    91 => { ent.shadow_map_size = pair.assert_i32()?; },
                    280 => { ent.shadow_map_softness = pair.assert_i16()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Line(ref mut ent) => {
                match pair.code {
                    39 => { ent.thickness = pair.assert_f64()?; },
                    10 => { ent.p1.x = pair.assert_f64()?; },
                    20 => { ent.p1.y = pair.assert_f64()?; },
                    30 => { ent.p1.z = pair.assert_f64()?; },
                    11 => { ent.p2.x = pair.assert_f64()?; },
                    21 => { ent.p2.y = pair.assert_f64()?; },
                    31 => { ent.p2.z = pair.assert_f64()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::LwPolyline(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::MLine(ref mut ent) => {
                match pair.code {
                    2 => { ent.style_name = pair.assert_string()?; },
                    340 => { ent.__style_handle = pair.as_handle()?; },
                    40 => { ent.scale_factor = pair.assert_f64()?; },
                    70 => { ent.justification = enum_from_number!(Justification, Top, from_i16, pair.assert_i16()?); },
                    71 => { ent.flags = pair.assert_i16()? as i32; },
                    72 => { ent.__vertex_count = pair.assert_i16()? as i32; },
                    73 => { ent.style_element_count = pair.assert_i16()? as i32; },
                    10 => { ent.start_point.x = pair.assert_f64()?; },
                    20 => { ent.start_point.y = pair.assert_f64()?; },
                    30 => { ent.start_point.z = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    11 => { ent.__vertices_x.push(pair.assert_f64()?); },
                    21 => { ent.__vertices_y.push(pair.assert_f64()?); },
                    31 => { ent.__vertices_z.push(pair.assert_f64()?); },
                    12 => { ent.__segment_direction_x.push(pair.assert_f64()?); },
                    22 => { ent.__segment_direction_y.push(pair.assert_f64()?); },
                    32 => { ent.__segment_direction_z.push(pair.assert_f64()?); },
                    13 => { ent.__miter_direction_x.push(pair.assert_f64()?); },
                    23 => { ent.__miter_direction_y.push(pair.assert_f64()?); },
                    33 => { ent.__miter_direction_z.push(pair.assert_f64()?); },
                    74 => { ent.__parameter_count = pair.assert_i16()? as i32; },
                    41 => { ent.parameters.push(pair.assert_f64()?); },
                    75 => { ent.__area_fill_parameter_count = pair.assert_i16()? as i32; },
                    42 => { ent.area_fill_parameters.push(pair.assert_f64()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::MText(_) => { panic!("this case should have been covered in a custom reader"); },
            &mut EntityType::OleFrame(ref mut ent) => {
                match pair.code {
                    70 => { ent.version_number = pair.assert_i16()? as i32; },
                    90 => { ent.binary_data_length = pair.assert_i32()?; },
                    310 => { ent.binary_data_strings.push(pair.assert_binary()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Ole2Frame(ref mut ent) => {
                match pair.code {
                    70 => { ent.version_number = pair.assert_i16()? as i32; },
                    3 => { ent.description = pair.assert_string()?; },
                    10 => { ent.upper_left_corner.x = pair.assert_f64()?; },
                    20 => { ent.upper_left_corner.y = pair.assert_f64()?; },
                    30 => { ent.upper_left_corner.z = pair.assert_f64()?; },
                    11 => { ent.lower_right_corner.x = pair.assert_f64()?; },
                    21 => { ent.lower_right_corner.y = pair.assert_f64()?; },
                    31 => { ent.lower_right_corner.z = pair.assert_f64()?; },
                    71 => { ent.object_type = enum_from_number!(OleObjectType, Static, from_i16, pair.assert_i16()?); },
                    72 => { ent.tile_mode = enum_from_number!(TileModeDescriptor, InTiledViewport, from_i16, pair.assert_i16()?); },
                    90 => { ent.binary_data_length = pair.assert_i32()?; },
                    310 => { ent.binary_data_strings.push(pair.assert_binary()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::ModelPoint(ref mut ent) => {
                match pair.code {
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    39 => { ent.thickness = pair.assert_f64()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    50 => { ent.angle = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Polyline(ref mut ent) => {
                match pair.code {
                    66 => { ent.contains_vertices = as_bool(pair.assert_i16()?); },
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    39 => { ent.thickness = pair.assert_f64()?; },
                    70 => { ent.flags = pair.assert_i16()? as i32; },
                    40 => { ent.default_starting_width = pair.assert_f64()?; },
                    41 => { ent.default_ending_width = pair.assert_f64()?; },
                    71 => { ent.polygon_mesh_m_vertex_count = pair.assert_i16()? as i32; },
                    72 => { ent.polygon_mesh_n_vertex_count = pair.assert_i16()? as i32; },
                    73 => { ent.smooth_surface_m_density = pair.assert_i16()? as i32; },
                    74 => { ent.smooth_surface_n_density = pair.assert_i16()? as i32; },
                    75 => { ent.surface_type = enum_from_number!(PolylineCurvedAndSmoothSurfaceType, None, from_i16, pair.assert_i16()?); },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Ray(ref mut ent) => {
                match pair.code {
                    10 => { ent.start_point.x = pair.assert_f64()?; },
                    20 => { ent.start_point.y = pair.assert_f64()?; },
                    30 => { ent.start_point.z = pair.assert_f64()?; },
                    11 => { ent.unit_direction_vector.x = pair.assert_f64()?; },
                    21 => { ent.unit_direction_vector.y = pair.assert_f64()?; },
                    31 => { ent.unit_direction_vector.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Region(ref mut ent) => {
                match pair.code {
                    70 => { ent.format_version_number = pair.assert_i16()?; },
                    1 => { ent.custom_data.push(pair.assert_string()?); },
                    3 => { ent.custom_data2.push(pair.assert_string()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::RText(ref mut ent) => {
                match pair.code {
                    10 => { ent.insertion_point.x = pair.assert_f64()?; },
                    20 => { ent.insertion_point.y = pair.assert_f64()?; },
                    30 => { ent.insertion_point.z = pair.assert_f64()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    50 => { ent.rotation_angle = pair.assert_f64()?; },
                    40 => { ent.text_height = pair.assert_f64()?; },
                    7 => { ent.text_style = pair.assert_string()?; },
                    70 => { ent.type_flags = pair.assert_i16()? as i32; },
                    1 => { ent.contents = pair.assert_string()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Section(ref mut ent) => {
                match pair.code {
                    90 => { ent.state = pair.assert_i32()?; },
                    91 => { ent.flags = pair.assert_i32()?; },
                    1 => { ent.name = pair.assert_string()?; },
                    10 => { ent.vertical_direction.x = pair.assert_f64()?; },
                    20 => { ent.vertical_direction.y = pair.assert_f64()?; },
                    30 => { ent.vertical_direction.z = pair.assert_f64()?; },
                    40 => { ent.top_height = pair.assert_f64()?; },
                    41 => { ent.bottom_height = pair.assert_f64()?; },
                    70 => { ent.indicator_transparency = pair.assert_i16()?; },
                    63 => { ent.indicator_color = Color::from_raw_value(pair.assert_i16()?); },
                    411 => { ent.indicator_color_name = pair.assert_string()?; },
                    92 => { ent.__vertex_count = pair.assert_i32()?; },
                    11 => { ent.__vertices_x.push(pair.assert_f64()?); },
                    21 => { ent.__vertices_y.push(pair.assert_f64()?); },
                    31 => { ent.__vertices_z.push(pair.assert_f64()?); },
                    93 => { ent.__back_line_vertex_count = pair.assert_i32()?; },
                    12 => { ent.__back_line_vertices_x.push(pair.assert_f64()?); },
                    22 => { ent.__back_line_vertices_y.push(pair.assert_f64()?); },
                    32 => { ent.__back_line_vertices_z.push(pair.assert_f64()?); },
                    360 => { ent.__geometry_settings_handle = pair.as_handle()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Seqend(ref mut _ent) => {
                match pair.code {
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Shape(ref mut ent) => {
                match pair.code {
                    39 => { ent.thickness = pair.assert_f64()?; },
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    40 => { ent.size = pair.assert_f64()?; },
                    2 => { ent.name = pair.assert_string()?; },
                    50 => { ent.rotation_angle = pair.assert_f64()?; },
                    41 => { ent.relative_x_scale_factor = pair.assert_f64()?; },
                    51 => { ent.oblique_angle = pair.assert_f64()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Solid(ref mut ent) => {
                match pair.code {
                    10 => { ent.first_corner.x = pair.assert_f64()?; },
                    20 => { ent.first_corner.y = pair.assert_f64()?; },
                    30 => { ent.first_corner.z = pair.assert_f64()?; },
                    11 => { ent.second_corner.x = pair.assert_f64()?; },
                    21 => { ent.second_corner.y = pair.assert_f64()?; },
                    31 => { ent.second_corner.z = pair.assert_f64()?; },
                    12 => { ent.third_corner.x = pair.assert_f64()?; },
                    22 => { ent.third_corner.y = pair.assert_f64()?; },
                    32 => { ent.third_corner.z = pair.assert_f64()?; },
                    13 => { ent.fourth_corner.x = pair.assert_f64()?; },
                    23 => { ent.fourth_corner.y = pair.assert_f64()?; },
                    33 => { ent.fourth_corner.z = pair.assert_f64()?; },
                    39 => { ent.thickness = pair.assert_f64()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Spline(ref mut ent) => {
                match pair.code {
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    70 => { ent.flags = pair.assert_i16()? as i32; },
                    71 => { ent.degree_of_curve = pair.assert_i16()? as i32; },
                    72 => { ent.__number_of_knots_ignored = pair.assert_i16()? as i32; },
                    73 => { ent.__number_of_control_points_ignored = pair.assert_i16()? as i32; },
                    74 => { ent.__number_of_fit_points_ignored = pair.assert_i16()? as i32; },
                    42 => { ent.knot_tolerance = pair.assert_f64()?; },
                    43 => { ent.control_point_tolerance = pair.assert_f64()?; },
                    44 => { ent.fit_tolerance = pair.assert_f64()?; },
                    12 => { ent.start_tangent.x = pair.assert_f64()?; },
                    22 => { ent.start_tangent.y = pair.assert_f64()?; },
                    32 => { ent.start_tangent.z = pair.assert_f64()?; },
                    13 => { ent.end_tangent.x = pair.assert_f64()?; },
                    23 => { ent.end_tangent.y = pair.assert_f64()?; },
                    33 => { ent.end_tangent.z = pair.assert_f64()?; },
                    40 => { ent.knot_values.push(pair.assert_f64()?); },
                    41 => { ent.weight = pair.assert_f64()?; },
                    10 => { ent.__control_point_x.push(pair.assert_f64()?); },
                    20 => { ent.__control_point_y.push(pair.assert_f64()?); },
                    30 => { ent.__control_point_z.push(pair.assert_f64()?); },
                    11 => { ent.__fit_point_x.push(pair.assert_f64()?); },
                    21 => { ent.__fit_point_y.push(pair.assert_f64()?); },
                    31 => { ent.__fit_point_z.push(pair.assert_f64()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Text(ref mut ent) => {
                match pair.code {
                    39 => { ent.thickness = pair.assert_f64()?; },
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    40 => { ent.text_height = pair.assert_f64()?; },
                    1 => { ent.value = pair.assert_string()?; },
                    50 => { ent.rotation = pair.assert_f64()?; },
                    41 => { ent.relative_x_scale_factor = pair.assert_f64()?; },
                    51 => { ent.oblique_angle = pair.assert_f64()?; },
                    7 => { ent.text_style_name = pair.assert_string()?; },
                    71 => { ent.text_generation_flags = pair.assert_i16()? as i32; },
                    72 => { ent.horizontal_text_justification = enum_from_number!(HorizontalTextJustification, Left, from_i16, pair.assert_i16()?); },
                    11 => { ent.second_alignment_point.x = pair.assert_f64()?; },
                    21 => { ent.second_alignment_point.y = pair.assert_f64()?; },
                    31 => { ent.second_alignment_point.z = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    73 => { ent.vertical_text_justification = enum_from_number!(VerticalTextJustification, Baseline, from_i16, pair.assert_i16()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Tolerance(ref mut ent) => {
                match pair.code {
                    3 => { ent.dimension_style_name = pair.assert_string()?; },
                    10 => { ent.insertion_point.x = pair.assert_f64()?; },
                    20 => { ent.insertion_point.y = pair.assert_f64()?; },
                    30 => { ent.insertion_point.z = pair.assert_f64()?; },
                    1 => { ent.display_text = pair.assert_string()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    11 => { ent.direction_vector.x = pair.assert_f64()?; },
                    21 => { ent.direction_vector.y = pair.assert_f64()?; },
                    31 => { ent.direction_vector.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Trace(ref mut ent) => {
                match pair.code {
                    10 => { ent.first_corner.x = pair.assert_f64()?; },
                    20 => { ent.first_corner.y = pair.assert_f64()?; },
                    30 => { ent.first_corner.z = pair.assert_f64()?; },
                    11 => { ent.second_corner.x = pair.assert_f64()?; },
                    21 => { ent.second_corner.y = pair.assert_f64()?; },
                    31 => { ent.second_corner.z = pair.assert_f64()?; },
                    12 => { ent.third_corner.x = pair.assert_f64()?; },
                    22 => { ent.third_corner.y = pair.assert_f64()?; },
                    32 => { ent.third_corner.z = pair.assert_f64()?; },
                    13 => { ent.fourth_corner.x = pair.assert_f64()?; },
                    23 => { ent.fourth_corner.y = pair.assert_f64()?; },
                    33 => { ent.fourth_corner.z = pair.assert_f64()?; },
                    39 => { ent.thickness = pair.assert_f64()?; },
                    210 => { ent.extrusion_direction.x = pair.assert_f64()?; },
                    220 => { ent.extrusion_direction.y = pair.assert_f64()?; },
                    230 => { ent.extrusion_direction.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::DgnUnderlay(ref mut ent) => {
                match pair.code {
                    340 => { ent.__object_handle = pair.as_handle()?; },
                    10 => { ent.insertion_point.x = pair.assert_f64()?; },
                    20 => { ent.insertion_point.y = pair.assert_f64()?; },
                    30 => { ent.insertion_point.z = pair.assert_f64()?; },
                    41 => { ent.x_scale = pair.assert_f64()?; },
                    42 => { ent.y_scale = pair.assert_f64()?; },
                    43 => { ent.z_scale = pair.assert_f64()?; },
                    50 => { ent.rotation_angle = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    280 => { ent.flags = pair.assert_i16()? as i32; },
                    281 => { ent.contrast = pair.assert_i16()?; },
                    282 => { ent.fade = pair.assert_i16()?; },
                    11 => { ent.__point_x.push(pair.assert_f64()?); },
                    21 => { ent.__point_y.push(pair.assert_f64()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::DwfUnderlay(ref mut ent) => {
                match pair.code {
                    340 => { ent.__object_handle = pair.as_handle()?; },
                    10 => { ent.insertion_point.x = pair.assert_f64()?; },
                    20 => { ent.insertion_point.y = pair.assert_f64()?; },
                    30 => { ent.insertion_point.z = pair.assert_f64()?; },
                    41 => { ent.x_scale = pair.assert_f64()?; },
                    42 => { ent.y_scale = pair.assert_f64()?; },
                    43 => { ent.z_scale = pair.assert_f64()?; },
                    50 => { ent.rotation_angle = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    280 => { ent.flags = pair.assert_i16()? as i32; },
                    281 => { ent.contrast = pair.assert_i16()?; },
                    282 => { ent.fade = pair.assert_i16()?; },
                    11 => { ent.__point_x.push(pair.assert_f64()?); },
                    21 => { ent.__point_y.push(pair.assert_f64()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::PdfUnderlay(ref mut ent) => {
                match pair.code {
                    340 => { ent.__object_handle = pair.as_handle()?; },
                    10 => { ent.insertion_point.x = pair.assert_f64()?; },
                    20 => { ent.insertion_point.y = pair.assert_f64()?; },
                    30 => { ent.insertion_point.z = pair.assert_f64()?; },
                    41 => { ent.x_scale = pair.assert_f64()?; },
                    42 => { ent.y_scale = pair.assert_f64()?; },
                    43 => { ent.z_scale = pair.assert_f64()?; },
                    50 => { ent.rotation_angle = pair.assert_f64()?; },
                    210 => { ent.normal.x = pair.assert_f64()?; },
                    220 => { ent.normal.y = pair.assert_f64()?; },
                    230 => { ent.normal.z = pair.assert_f64()?; },
                    280 => { ent.flags = pair.assert_i16()? as i32; },
                    281 => { ent.contrast = pair.assert_i16()?; },
                    282 => { ent.fade = pair.assert_i16()?; },
                    11 => { ent.__point_x.push(pair.assert_f64()?); },
                    21 => { ent.__point_y.push(pair.assert_f64()?); },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Vertex(ref mut ent) => {
                match pair.code {
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    40 => { ent.starting_width = pair.assert_f64()?; },
                    41 => { ent.ending_width = pair.assert_f64()?; },
                    42 => { ent.bulge = pair.assert_f64()?; },
                    70 => { ent.flags = pair.assert_i16()? as i32; },
                    50 => { ent.curve_fit_tangent_direction = pair.assert_f64()?; },
                    71 => { ent.polyface_mesh_vertex_index1 = pair.assert_i16()? as i32; },
                    72 => { ent.polyface_mesh_vertex_index2 = pair.assert_i16()? as i32; },
                    73 => { ent.polyface_mesh_vertex_index3 = pair.assert_i16()? as i32; },
                    74 => { ent.polyface_mesh_vertex_index4 = pair.assert_i16()? as i32; },
                    91 => { ent.identifier = pair.assert_i32()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::Wipeout(ref mut ent) => {
                match pair.code {
                    90 => { ent.class_version = pair.assert_i32()?; },
                    10 => { ent.location.x = pair.assert_f64()?; },
                    20 => { ent.location.y = pair.assert_f64()?; },
                    30 => { ent.location.z = pair.assert_f64()?; },
                    11 => { ent.u_vector.x = pair.assert_f64()?; },
                    21 => { ent.u_vector.y = pair.assert_f64()?; },
                    31 => { ent.u_vector.z = pair.assert_f64()?; },
                    12 => { ent.v_vector.x = pair.assert_f64()?; },
                    22 => { ent.v_vector.y = pair.assert_f64()?; },
                    32 => { ent.v_vector.z = pair.assert_f64()?; },
                    13 => { ent.image_size.x = pair.assert_f64()?; },
                    23 => { ent.image_size.y = pair.assert_f64()?; },
                    340 => { ent.image_def_reference = pair.assert_string()?; },
                    70 => { ent.display_options_flags = pair.assert_i16()? as i32; },
                    280 => { ent.use_clipping = as_bool(pair.assert_i16()?); },
                    281 => { ent.brightness = pair.assert_i16()?; },
                    282 => { ent.contrast = pair.assert_i16()?; },
                    283 => { ent.fade = pair.assert_i16()?; },
                    360 => { ent.image_def_reactor_reference = pair.assert_string()?; },
                    71 => { ent.clipping_type = enum_from_number!(ImageClippingBoundaryType, Rectangular, from_i16, pair.assert_i16()?); },
                    91 => { ent.clipping_vertex_count = pair.assert_i32()?; },
                    14 => { ent.__clipping_vertices_x.push(pair.assert_f64()?); },
                    290 => { ent.is_inside_clipping = pair.assert_bool()?; },
                    _ => return Ok(false),
                }
            },
            &mut EntityType::XLine(ref mut ent) => {
                match pair.code {
                    10 => { ent.first_point.x = pair.assert_f64()?; },
                    20 => { ent.first_point.y = pair.assert_f64()?; },
                    30 => { ent.first_point.z = pair.assert_f64()?; },
                    11 => { ent.unit_direction_vector.x = pair.assert_f64()?; },
                    21 => { ent.unit_direction_vector.y = pair.assert_f64()?; },
                    31 => { ent.unit_direction_vector.z = pair.assert_f64()?; },
                    _ => return Ok(false),
                }
            },
        }
        Ok(true)
    }
    pub(crate) fn write<T>(&self, common: &EntityCommon, version: AcadVersion, writer: &mut CodePairWriter<T>) -> DxfResult<()>
        where T: Write + ?Sized {

        match self {
            &EntityType::Face3D(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbFace"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.first_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.first_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.first_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.second_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.second_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.second_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(12, ent.third_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(22, ent.third_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(32, ent.third_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(13, ent.fourth_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(23, ent.fourth_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(33, ent.fourth_corner.z))?;
                if ent.edge_flags != 0 {
                    writer.write_code_pair(&CodePair::new_i16(70, ent.edge_flags as i16))?;
                }
            },
            &EntityType::Solid3D(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbModelerGeometry")))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.format_version_number))?;
                for v in &ent.custom_data {
                    writer.write_code_pair(&CodePair::new_string(1, &v))?;
                }
                for v in &ent.custom_data2 {
                    writer.write_code_pair(&CodePair::new_string(3, &v))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDb3dSolid")))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_string(350, &ent.__history_object_handle.as_string()))?;
                }
            },
            &EntityType::ProxyEntity(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbProxyEntity"))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.proxy_entity_class_id))?;
                writer.write_code_pair(&CodePair::new_i32(91, ent.application_entity_class_id))?;
                writer.write_code_pair(&CodePair::new_i32(92, ent.graphics_data_size))?;
                for v in &ent.graphics_data_string {
                    writer.write_code_pair(&CodePair::new_binary(310, v.clone()))?;
                }
                writer.write_code_pair(&CodePair::new_i32(93, ent.entity_data_size))?;
                for v in &ent.entity_data_string {
                    writer.write_code_pair(&CodePair::new_binary(310, v.clone()))?;
                }
                for v in &ent.object_id_1 {
                    writer.write_code_pair(&CodePair::new_string(330, &v))?;
                }
                for v in &ent.object_id_2 {
                    writer.write_code_pair(&CodePair::new_string(340, &v))?;
                }
                for v in &ent.object_id_3 {
                    writer.write_code_pair(&CodePair::new_string(350, &v))?;
                }
                for v in &ent.object_id_4 {
                    writer.write_code_pair(&CodePair::new_string(360, &v))?;
                }
                writer.write_code_pair(&CodePair::new_i32(94, ent.terminator))?;
                if version >= AcadVersion::R2000 {
                    writer.write_code_pair(&CodePair::new_i32(95, ent.__object_drawing_format as i32))?;
                }
                if version >= AcadVersion::R2000 {
                    writer.write_code_pair(&CodePair::new_i16(70, as_i16(ent.original_data_format_is_dxf)))?;
                }
            },
            &EntityType::Arc(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbArc"))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                writer.write_code_pair(&CodePair::new_f64(10, ent.center.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.center.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.center.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.radius))?;
                if ent.normal != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                }
                writer.write_code_pair(&CodePair::new_f64(50, ent.start_angle))?;
                writer.write_code_pair(&CodePair::new_f64(51, ent.end_angle))?;
            },
            &EntityType::ArcAlignedText(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbArcAlignedText"))?;
                writer.write_code_pair(&CodePair::new_string(1, &ent.text))?;
                writer.write_code_pair(&CodePair::new_string(2, &ent.font_name))?;
                writer.write_code_pair(&CodePair::new_string(3, &ent.bigfont_name))?;
                writer.write_code_pair(&CodePair::new_string(7, &ent.text_style_name))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.center_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.center_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.center_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.arc_radius))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.width_factor))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.text_height))?;
                writer.write_code_pair(&CodePair::new_f64(43, ent.character_spacing))?;
                writer.write_code_pair(&CodePair::new_f64(44, ent.offset_from_arc))?;
                writer.write_code_pair(&CodePair::new_f64(45, ent.right_offset))?;
                writer.write_code_pair(&CodePair::new_f64(46, ent.left_offset))?;
                writer.write_code_pair(&CodePair::new_f64(50, ent.start_angle))?;
                writer.write_code_pair(&CodePair::new_f64(51, ent.end_angle))?;
                writer.write_code_pair(&CodePair::new_i16(70, as_i16(ent.is_character_order_reversed)))?;
                writer.write_code_pair(&CodePair::new_i16(71, ent.direction_flag))?;
                writer.write_code_pair(&CodePair::new_i16(72, ent.alignment_flag))?;
                writer.write_code_pair(&CodePair::new_i16(73, ent.side_flag))?;
                writer.write_code_pair(&CodePair::new_i16(74, as_i16(ent.is_bold)))?;
                writer.write_code_pair(&CodePair::new_i16(75, as_i16(ent.is_italic)))?;
                writer.write_code_pair(&CodePair::new_i16(76, as_i16(ent.is_underline)))?;
                writer.write_code_pair(&CodePair::new_i16(77, ent.character_set_value))?;
                writer.write_code_pair(&CodePair::new_i16(78, ent.pitch_and_family_value))?;
                writer.write_code_pair(&CodePair::new_i16(79, ent.font_type as i16))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.color_index))?;
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
                writer.write_code_pair(&CodePair::new_i16(280, ent.wizard_flag))?;
            },
            &EntityType::AttributeDefinition(ref ent) => {
                if version >= AcadVersion::R13 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbText")))?;
                }
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.text_height))?;
                writer.write_code_pair(&CodePair::new_string(1, &ent.value))?;
                if version >= AcadVersion::R13 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbAttributeDefinition")))?;
                }
                if ent.rotation != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(50, ent.rotation))?;
                }
                if ent.relative_x_scale_factor != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(41, ent.relative_x_scale_factor))?;
                }
                if ent.oblique_angle != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(51, ent.oblique_angle))?;
                }
                if ent.text_style_name != String::from("STANDARD") {
                    writer.write_code_pair(&CodePair::new_string(7, &ent.text_style_name))?;
                }
                if ent.text_generation_flags != 0 {
                    writer.write_code_pair(&CodePair::new_i16(71, ent.text_generation_flags as i16))?;
                }
                if ent.horizontal_text_justification != HorizontalTextJustification::Left {
                    writer.write_code_pair(&CodePair::new_i16(72, ent.horizontal_text_justification as i16))?;
                }
                writer.write_code_pair(&CodePair::new_f64(11, ent.second_alignment_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.second_alignment_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.second_alignment_point.z))?;
                if ent.normal != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                }
                if version >= AcadVersion::R13 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbAttributeDefinition")))?;
                }
                if version >= AcadVersion::R2010 {
                    writer.write_code_pair(&CodePair::new_i16(280, ent.version as i16))?;
                }
                writer.write_code_pair(&CodePair::new_string(3, &ent.prompt))?;
                writer.write_code_pair(&CodePair::new_string(2, &ent.text_tag))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.flags as i16))?;
                if ent.field_length != 0 {
                    writer.write_code_pair(&CodePair::new_i16(73, ent.field_length))?;
                }
                if version >= AcadVersion::R12 && ent.vertical_text_justification != VerticalTextJustification::Baseline {
                    writer.write_code_pair(&CodePair::new_i16(74, ent.vertical_text_justification as i16))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(280, as_i16(ent.is_locked_in_block)))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbXrecord")))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(280, as_i16(ent.keep_duplicate_records)))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(70, ent.m_text_flag as i16))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(70, as_i16(ent.is_really_locked)))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(70, ent.__secondary_attributes_handle.len() as i16))?;
                }
                if version >= AcadVersion::R2007 {
                    for v in &ent.__secondary_attributes_handle {
                        writer.write_code_pair(&CodePair::new_string(340, &v.as_string()))?;
                    }
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_f64(10, ent.alignment_point.x))?;
                    writer.write_code_pair(&CodePair::new_f64(20, ent.alignment_point.y))?;
                    writer.write_code_pair(&CodePair::new_f64(30, ent.alignment_point.z))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_f64(40, ent.annotation_scale))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_string(2, &ent.x_record_tag))?;
                }
            },
            &EntityType::Attribute(ref ent) => {
                if version >= AcadVersion::R13 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbText")))?;
                }
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.text_height))?;
                writer.write_code_pair(&CodePair::new_string(1, &ent.value))?;
                if version >= AcadVersion::R13 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbAttribute")))?;
                }
                if version >= AcadVersion::R2010 {
                    writer.write_code_pair(&CodePair::new_i16(280, ent.version as i16))?;
                }
                writer.write_code_pair(&CodePair::new_string(2, &ent.attribute_tag))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.flags as i16))?;
                if ent.field_length != 0 {
                    writer.write_code_pair(&CodePair::new_i16(73, ent.field_length))?;
                }
                if ent.rotation != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(50, ent.rotation))?;
                }
                if ent.relative_x_scale_factor != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(41, ent.relative_x_scale_factor))?;
                }
                if ent.oblique_angle != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(51, ent.oblique_angle))?;
                }
                if ent.text_style_name != String::from("STANDARD") {
                    writer.write_code_pair(&CodePair::new_string(7, &ent.text_style_name))?;
                }
                if ent.text_generation_flags != 0 {
                    writer.write_code_pair(&CodePair::new_i16(71, ent.text_generation_flags as i16))?;
                }
                if ent.horizontal_text_justification != HorizontalTextJustification::Left {
                    writer.write_code_pair(&CodePair::new_i16(72, ent.horizontal_text_justification as i16))?;
                }
                if version >= AcadVersion::R12 && ent.vertical_text_justification != VerticalTextJustification::Baseline {
                    writer.write_code_pair(&CodePair::new_i16(74, ent.vertical_text_justification as i16))?;
                }
                writer.write_code_pair(&CodePair::new_f64(11, ent.second_alignment_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.second_alignment_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.second_alignment_point.z))?;
                if ent.normal != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(280, as_i16(ent.is_locked_in_block)))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbXrecord")))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(280, as_i16(ent.keep_duplicate_records)))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(70, ent.m_text_flag as i16))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(70, as_i16(ent.is_really_locked)))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_i16(70, ent.__secondary_attributes_handle.len() as i16))?;
                }
                if version >= AcadVersion::R2007 {
                    for v in &ent.__secondary_attributes_handle {
                        writer.write_code_pair(&CodePair::new_string(340, &v.as_string()))?;
                    }
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_f64(10, ent.alignment_point.x))?;
                    writer.write_code_pair(&CodePair::new_f64(20, ent.alignment_point.y))?;
                    writer.write_code_pair(&CodePair::new_f64(30, ent.alignment_point.z))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_f64(40, ent.annotation_scale))?;
                }
                if version >= AcadVersion::R2007 {
                    writer.write_code_pair(&CodePair::new_string(2, &ent.x_record_tag))?;
                }
            },
            &EntityType::Body(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbModelerGeometry"))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.format_version_number))?;
                for v in &ent.custom_data {
                    writer.write_code_pair(&CodePair::new_string(1, &v))?;
                }
                for v in &ent.custom_data2 {
                    writer.write_code_pair(&CodePair::new_string(3, &v))?;
                }
            },
            &EntityType::Circle(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbCircle"))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                writer.write_code_pair(&CodePair::new_f64(10, ent.center.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.center.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.center.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.radius))?;
                if ent.normal != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                }
            },
            &EntityType::RotatedDimension(_) => { panic!("this case should have been covered in a custom writer"); },
            &EntityType::RadialDimension(_) => { panic!("this case should have been covered in a custom writer"); },
            &EntityType::DiameterDimension(_) => { panic!("this case should have been covered in a custom writer"); },
            &EntityType::AngularThreePointDimension(_) => { panic!("this case should have been covered in a custom writer"); },
            &EntityType::OrdinateDimension(_) => { panic!("this case should have been covered in a custom writer"); },
            &EntityType::Ellipse(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbEllipse"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.center.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.center.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.center.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.major_axis.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.major_axis.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.major_axis.z))?;
                if ent.normal != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                }
                writer.write_code_pair(&CodePair::new_f64(40, ent.minor_axis_ratio))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.start_parameter))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.end_parameter))?;
            },
            &EntityType::Helix(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbHelix"))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.major_release_number))?;
                writer.write_code_pair(&CodePair::new_i32(91, ent.maintainence_release_number))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.axis_base_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.axis_base_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.axis_base_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.start_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.start_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.start_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(12, ent.axis_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(22, ent.axis_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(32, ent.axis_vector.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.radius))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.number_of_turns))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.turn_height))?;
                writer.write_code_pair(&CodePair::new_bool(290, ent.is_right_handed))?;
                writer.write_code_pair(&CodePair::new_i16(280, ent.constraint as i16))?;
            },
            &EntityType::Image(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbRasterImage")))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.class_version))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.u_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.u_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.u_vector.z))?;
                writer.write_code_pair(&CodePair::new_f64(12, ent.v_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(22, ent.v_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(32, ent.v_vector.z))?;
                writer.write_code_pair(&CodePair::new_f64(13, ent.image_size.x))?;
                writer.write_code_pair(&CodePair::new_f64(23, ent.image_size.y))?;
                writer.write_code_pair(&CodePair::new_string(340, &ent.image_def_reference))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.display_options_flags as i16))?;
                writer.write_code_pair(&CodePair::new_i16(280, as_i16(ent.use_clipping)))?;
                writer.write_code_pair(&CodePair::new_i16(281, ent.brightness))?;
                writer.write_code_pair(&CodePair::new_i16(282, ent.contrast))?;
                writer.write_code_pair(&CodePair::new_i16(283, ent.fade))?;
                writer.write_code_pair(&CodePair::new_string(360, &ent.image_def_reactor_reference))?;
                writer.write_code_pair(&CodePair::new_i16(71, ent.clipping_type as i16))?;
                writer.write_code_pair(&CodePair::new_i32(91, ent.clipping_vertices.len() as i32))?;
                for item in &ent.clipping_vertices {
                    writer.write_code_pair(&CodePair::new_f64(14, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(24, item.y))?;
                }
                if version >= AcadVersion::R2010 {
                    writer.write_code_pair(&CodePair::new_bool(290, ent.is_inside_clipping))?;
                }
            },
            &EntityType::Insert(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbBlockReference")))?;
                writer.write_code_pair(&CodePair::new_i16(66, as_i16(ent.attributes().count() > 0)))?;
                writer.write_code_pair(&CodePair::new_string(2, &ent.name))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                if ent.x_scale_factor != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(41, ent.x_scale_factor))?;
                }
                if ent.y_scale_factor != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(42, ent.y_scale_factor))?;
                }
                if ent.z_scale_factor != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(43, ent.z_scale_factor))?;
                }
                if ent.rotation != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(50, ent.rotation))?;
                }
                if ent.column_count != 1 {
                    writer.write_code_pair(&CodePair::new_i16(70, ent.column_count))?;
                }
                if ent.row_count != 1 {
                    writer.write_code_pair(&CodePair::new_i16(71, ent.row_count))?;
                }
                if ent.column_spacing != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(44, ent.column_spacing))?;
                }
                if ent.row_spacing != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(45, ent.row_spacing))?;
                }
                writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
            },
            &EntityType::Leader(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbLeader")))?;
                writer.write_code_pair(&CodePair::new_string(3, &ent.dimension_style_name))?;
                writer.write_code_pair(&CodePair::new_i16(71, as_i16(ent.use_arrowheads)))?;
                writer.write_code_pair(&CodePair::new_i16(72, ent.path_type as i16))?;
                writer.write_code_pair(&CodePair::new_i16(73, ent.annotation_type as i16))?;
                writer.write_code_pair(&CodePair::new_i16(74, ent.hookline_direction as i16))?;
                writer.write_code_pair(&CodePair::new_i16(75, as_i16(ent.use_hookline)))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.text_annotation_height))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.text_annotation_width))?;
                writer.write_code_pair(&CodePair::new_i16(76, ent.vertices.len() as i16))?;
                for item in &ent.vertices {
                    writer.write_code_pair(&CodePair::new_f64(10, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(20, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(30, item.z))?;
                }
                if ent.override_color != Color::by_block() {
                    writer.write_code_pair(&CodePair::new_i16(77, ent.override_color.get_raw_value()))?;
                }
                writer.write_code_pair(&CodePair::new_string(340, &ent.associated_annotation_reference))?;
                writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                writer.write_code_pair(&CodePair::new_f64(211, ent.right.x))?;
                writer.write_code_pair(&CodePair::new_f64(221, ent.right.y))?;
                writer.write_code_pair(&CodePair::new_f64(231, ent.right.z))?;
                writer.write_code_pair(&CodePair::new_f64(212, ent.block_offset.x))?;
                writer.write_code_pair(&CodePair::new_f64(222, ent.block_offset.y))?;
                writer.write_code_pair(&CodePair::new_f64(232, ent.block_offset.z))?;
                if version >= AcadVersion::R14 {
                    writer.write_code_pair(&CodePair::new_f64(213, ent.annotation_offset.x))?;
                    writer.write_code_pair(&CodePair::new_f64(223, ent.annotation_offset.y))?;
                    writer.write_code_pair(&CodePair::new_f64(233, ent.annotation_offset.z))?;
                }
            },
            &EntityType::Light(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbLight"))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.version_number))?;
                writer.write_code_pair(&CodePair::new_string(1, &ent.name))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.light_type as i16))?;
                writer.write_code_pair(&CodePair::new_bool(290, ent.is_active))?;
                writer.write_code_pair(&CodePair::new_bool(291, ent.plot_glyph))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.intensity))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.position.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.position.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.position.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.target_location.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.target_location.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.target_location.z))?;
                writer.write_code_pair(&CodePair::new_i16(72, ent.attentuation_type as i16))?;
                writer.write_code_pair(&CodePair::new_bool(292, ent.use_attenuation_limits))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.attenuation_start_limit))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.attenuation_end_limit))?;
                writer.write_code_pair(&CodePair::new_f64(50, ent.hotspot_angle))?;
                writer.write_code_pair(&CodePair::new_f64(51, ent.falloff_angle))?;
                writer.write_code_pair(&CodePair::new_bool(293, ent.cast_shadows))?;
                writer.write_code_pair(&CodePair::new_i16(73, ent.shadow_type as i16))?;
                writer.write_code_pair(&CodePair::new_i32(91, ent.shadow_map_size))?;
                writer.write_code_pair(&CodePair::new_i16(280, ent.shadow_map_softness))?;
            },
            &EntityType::Line(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbLine"))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                writer.write_code_pair(&CodePair::new_f64(10, ent.p1.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.p1.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.p1.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.p2.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.p2.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.p2.z))?;
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
            },
            &EntityType::LwPolyline(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbPolyline")))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.vertices.len() as i32))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.flags as i16))?;
                if ent.constant_width != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(43, ent.constant_width))?;
                }
                if common.elevation != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(38, common.elevation))?;
                }
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                for item in &ent.vertices {
                    writer.write_code_pair(&CodePair::new_f64(10, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(20, item.y))?;
                    if version >= AcadVersion::R2013 {
                        writer.write_code_pair(&CodePair::new_i32(91, item.id))?;
                    }
                    if item.starting_width != 0.0 {
                        writer.write_code_pair(&CodePair::new_f64(40, item.starting_width))?;
                    }
                    if item.ending_width != 0.0 {
                        writer.write_code_pair(&CodePair::new_f64(41, item.ending_width))?;
                    }
                    if item.bulge != 0.0 {
                        writer.write_code_pair(&CodePair::new_f64(42, item.bulge))?;
                    }
                }
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
            },
            &EntityType::MLine(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbMline")))?;
                writer.write_code_pair(&CodePair::new_string(2, &ent.style_name))?;
                writer.write_code_pair(&CodePair::new_string(340, &ent.__style_handle.as_string()))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.scale_factor))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.justification as i16))?;
                writer.write_code_pair(&CodePair::new_i16(71, ent.flags as i16))?;
                writer.write_code_pair(&CodePair::new_i16(72, ent.vertices.len() as i16))?;
                writer.write_code_pair(&CodePair::new_i16(73, ent.style_element_count as i16))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.start_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.start_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.start_point.z))?;
                if ent.normal != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                }
                for item in &ent.vertices {
                    writer.write_code_pair(&CodePair::new_f64(10, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(20, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(30, item.z))?;
                }
                for item in &ent.segment_directions {
                    writer.write_code_pair(&CodePair::new_f64(11, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(21, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(31, item.z))?;
                }
                for item in &ent.miter_directions {
                    writer.write_code_pair(&CodePair::new_f64(12, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(22, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(32, item.z))?;
                }
                writer.write_code_pair(&CodePair::new_i16(74, ent.parameters.len() as i16))?;
                for v in &ent.parameters {
                    writer.write_code_pair(&CodePair::new_f64(41, *v))?;
                }
                writer.write_code_pair(&CodePair::new_i16(75, ent.area_fill_parameters.len() as i16))?;
                for v in &ent.area_fill_parameters {
                    writer.write_code_pair(&CodePair::new_f64(42, *v))?;
                }
            },
            &EntityType::MText(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbMText")))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.insertion_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.insertion_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.insertion_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.initial_text_height))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.reference_rectangle_width))?;
                writer.write_code_pair(&CodePair::new_i16(71, ent.attachment_point as i16))?;
                writer.write_code_pair(&CodePair::new_i16(72, ent.drawing_direction as i16))?;
                for v in &ent.extended_text {
                    writer.write_code_pair(&CodePair::new_string(3, &v))?;
                }
                writer.write_code_pair(&CodePair::new_string(1, &ent.text))?;
                if ent.text_style_name != String::from("STANDARD") {
                    writer.write_code_pair(&CodePair::new_string(7, &ent.text_style_name))?;
                }
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
                writer.write_code_pair(&CodePair::new_f64(11, ent.x_axis_direction.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.x_axis_direction.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.x_axis_direction.z))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.horizontal_width))?;
                writer.write_code_pair(&CodePair::new_f64(43, ent.vertical_height))?;
                writer.write_code_pair(&CodePair::new_f64(50, ent.rotation_angle))?;
                writer.write_code_pair(&CodePair::new_i16(73, ent.line_spacing_style as i16))?;
                writer.write_code_pair(&CodePair::new_f64(44, ent.line_spacing_factor))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.background_fill_setting as i32))?;
                writer.write_code_pair(&CodePair::new_i32(420, ent.background_color_rgb))?;
                writer.write_code_pair(&CodePair::new_string(430, &ent.background_color_name))?;
                if ent.fill_box_scale != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(45, ent.fill_box_scale))?;
                }
                writer.write_code_pair(&CodePair::new_i16(63, ent.background_fill_color.get_raw_value()))?;
                writer.write_code_pair(&CodePair::new_i32(441, ent.background_fill_color_transparency))?;
                writer.write_code_pair(&CodePair::new_i16(75, ent.column_type))?;
                writer.write_code_pair(&CodePair::new_i16(76, ent.column_count as i16))?;
                writer.write_code_pair(&CodePair::new_i16(78, as_i16(ent.is_column_flow_reversed)))?;
                writer.write_code_pair(&CodePair::new_i16(79, as_i16(ent.is_column_auto_height)))?;
                writer.write_code_pair(&CodePair::new_f64(48, ent.column_width))?;
                writer.write_code_pair(&CodePair::new_f64(49, ent.column_gutter))?;
                writer.write_code_pair(&CodePair::new_f64(50, ent.column_heights.len() as f64))?;
                for v in &ent.column_heights {
                    writer.write_code_pair(&CodePair::new_f64(50, *v))?;
                }
            },
            &EntityType::OleFrame(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbOleFrame")))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.version_number as i16))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.binary_data_length))?;
                for item in &ent.binary_data_strings {
                    writer.write_code_pair(&CodePair::new_binary(310, item.clone()))?;
                }
                writer.write_code_pair(&CodePair::new_string(1, &String::from("OLE")))?;
            },
            &EntityType::Ole2Frame(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbOle2Frame")))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.version_number as i16))?;
                writer.write_code_pair(&CodePair::new_string(3, &ent.description))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.upper_left_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.upper_left_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.upper_left_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.lower_right_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.lower_right_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.lower_right_corner.z))?;
                writer.write_code_pair(&CodePair::new_i16(71, ent.object_type as i16))?;
                writer.write_code_pair(&CodePair::new_i16(72, ent.tile_mode as i16))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.binary_data_length))?;
                for item in &ent.binary_data_strings {
                    writer.write_code_pair(&CodePair::new_binary(310, item.clone()))?;
                }
                writer.write_code_pair(&CodePair::new_string(1, &String::from("OLE")))?;
            },
            &EntityType::ModelPoint(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbPoint"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
                if ent.angle != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(50, ent.angle))?;
                }
            },
            &EntityType::Polyline(_) => { panic!("this case should have been covered in a custom writer"); },
            &EntityType::Ray(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbRay"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.start_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.start_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.start_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.unit_direction_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.unit_direction_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.unit_direction_vector.z))?;
            },
            &EntityType::Region(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbModelerGeometry"))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.format_version_number))?;
                for v in &ent.custom_data {
                    writer.write_code_pair(&CodePair::new_string(1, &v))?;
                }
                for v in &ent.custom_data2 {
                    writer.write_code_pair(&CodePair::new_string(3, &v))?;
                }
            },
            &EntityType::RText(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "RText"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.insertion_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.insertion_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.insertion_point.z))?;
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
                writer.write_code_pair(&CodePair::new_f64(50, ent.rotation_angle))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.text_height))?;
                writer.write_code_pair(&CodePair::new_string(7, &ent.text_style))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.type_flags as i16))?;
                writer.write_code_pair(&CodePair::new_string(1, &ent.contents))?;
            },
            &EntityType::Section(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbSection")))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.state))?;
                writer.write_code_pair(&CodePair::new_i32(91, ent.flags))?;
                writer.write_code_pair(&CodePair::new_string(1, &ent.name))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.vertical_direction.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.vertical_direction.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.vertical_direction.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.top_height))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.bottom_height))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.indicator_transparency))?;
                writer.write_code_pair(&CodePair::new_i16(63, ent.indicator_color.get_raw_value()))?;
                writer.write_code_pair(&CodePair::new_string(411, &ent.indicator_color_name))?;
                writer.write_code_pair(&CodePair::new_i32(92, ent.vertices.len() as i32))?;
                for item in &ent.vertices {
                    writer.write_code_pair(&CodePair::new_f64(11, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(21, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(31, item.z))?;
                }
                writer.write_code_pair(&CodePair::new_i32(93, ent.back_line_vertices.len() as i32))?;
                for item in &ent.back_line_vertices {
                    writer.write_code_pair(&CodePair::new_f64(12, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(22, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(32, item.z))?;
                }
                writer.write_code_pair(&CodePair::new_string(360, &ent.__geometry_settings_handle.as_string()))?;
            },
            &EntityType::Seqend(ref _ent) => {
            },
            &EntityType::Shape(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbShape"))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.size))?;
                writer.write_code_pair(&CodePair::new_string(2, &ent.name))?;
                if ent.rotation_angle != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(50, ent.rotation_angle))?;
                }
                if ent.relative_x_scale_factor != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(41, ent.relative_x_scale_factor))?;
                }
                if ent.oblique_angle != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(51, ent.oblique_angle))?;
                }
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
            },
            &EntityType::Solid(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbTrace"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.first_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.first_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.first_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.second_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.second_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.second_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(12, ent.third_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(22, ent.third_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(32, ent.third_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(13, ent.fourth_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(23, ent.fourth_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(33, ent.fourth_corner.z))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
            },
            &EntityType::Spline(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbSpline")))?;
                writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.flags as i16))?;
                writer.write_code_pair(&CodePair::new_i16(71, ent.degree_of_curve as i16))?;
                writer.write_code_pair(&CodePair::new_i16(72, ent.knot_values.len() as i16))?;
                writer.write_code_pair(&CodePair::new_i16(73, ent.control_points.len() as i16))?;
                writer.write_code_pair(&CodePair::new_i16(74, ent.fit_points.len() as i16))?;
                if ent.knot_tolerance != 0.000_000_1 {
                    writer.write_code_pair(&CodePair::new_f64(42, ent.knot_tolerance))?;
                }
                if ent.control_point_tolerance != 0.000_000_1 {
                    writer.write_code_pair(&CodePair::new_f64(43, ent.control_point_tolerance))?;
                }
                if ent.fit_tolerance != 0.000_000_000_1 {
                    writer.write_code_pair(&CodePair::new_f64(44, ent.fit_tolerance))?;
                }
                writer.write_code_pair(&CodePair::new_f64(12, ent.start_tangent.x))?;
                writer.write_code_pair(&CodePair::new_f64(22, ent.start_tangent.y))?;
                writer.write_code_pair(&CodePair::new_f64(32, ent.start_tangent.z))?;
                writer.write_code_pair(&CodePair::new_f64(13, ent.end_tangent.x))?;
                writer.write_code_pair(&CodePair::new_f64(23, ent.end_tangent.y))?;
                writer.write_code_pair(&CodePair::new_f64(33, ent.end_tangent.z))?;
                for v in &ent.knot_values {
                    writer.write_code_pair(&CodePair::new_f64(40, *v))?;
                }
                if ent.weight != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(41, ent.weight))?;
                }
                for item in &ent.control_points {
                    writer.write_code_pair(&CodePair::new_f64(10, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(20, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(30, item.z))?;
                }
                for item in &ent.fit_points {
                    writer.write_code_pair(&CodePair::new_f64(11, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(21, item.y))?;
                    writer.write_code_pair(&CodePair::new_f64(31, item.z))?;
                }
            },
            &EntityType::Text(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbText")))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                writer.write_code_pair(&CodePair::new_f64(40, ent.text_height))?;
                writer.write_code_pair(&CodePair::new_string(1, &ent.value))?;
                if ent.rotation != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(50, ent.rotation))?;
                }
                if ent.relative_x_scale_factor != 1.0 {
                    writer.write_code_pair(&CodePair::new_f64(41, ent.relative_x_scale_factor))?;
                }
                if ent.oblique_angle != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(51, ent.oblique_angle))?;
                }
                if ent.text_style_name != String::from("STANDARD") {
                    writer.write_code_pair(&CodePair::new_string(7, &ent.text_style_name))?;
                }
                if ent.text_generation_flags != 0 {
                    writer.write_code_pair(&CodePair::new_i16(71, ent.text_generation_flags as i16))?;
                }
                if ent.horizontal_text_justification != HorizontalTextJustification::Left {
                    writer.write_code_pair(&CodePair::new_i16(72, ent.horizontal_text_justification as i16))?;
                }
                if ent.second_alignment_point != Point::origin() {
                    writer.write_code_pair(&CodePair::new_f64(11, ent.second_alignment_point.x))?;
                    writer.write_code_pair(&CodePair::new_f64(21, ent.second_alignment_point.y))?;
                    writer.write_code_pair(&CodePair::new_f64(31, ent.second_alignment_point.z))?;
                }
                if ent.normal != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                }
                if ent.vertical_text_justification != VerticalTextJustification::Baseline {
                    writer.write_code_pair(&CodePair::new_i16(73, ent.vertical_text_justification as i16))?;
                }
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbText")))?;
            },
            &EntityType::Tolerance(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbFcf"))?;
                writer.write_code_pair(&CodePair::new_string(3, &ent.dimension_style_name))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.insertion_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.insertion_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.insertion_point.z))?;
                if version >= AcadVersion::R2000 {
                    writer.write_code_pair(&CodePair::new_string(1, &ent.display_text))?;
                }
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
                writer.write_code_pair(&CodePair::new_f64(11, ent.direction_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.direction_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.direction_vector.z))?;
            },
            &EntityType::Trace(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbTrace"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.first_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.first_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.first_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.second_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.second_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.second_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(12, ent.third_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(22, ent.third_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(32, ent.third_corner.z))?;
                writer.write_code_pair(&CodePair::new_f64(13, ent.fourth_corner.x))?;
                writer.write_code_pair(&CodePair::new_f64(23, ent.fourth_corner.y))?;
                writer.write_code_pair(&CodePair::new_f64(33, ent.fourth_corner.z))?;
                if ent.thickness != 0.0 {
                    writer.write_code_pair(&CodePair::new_f64(39, ent.thickness))?;
                }
                if ent.extrusion_direction != Vector::z_axis() {
                    writer.write_code_pair(&CodePair::new_f64(210, ent.extrusion_direction.x))?;
                    writer.write_code_pair(&CodePair::new_f64(220, ent.extrusion_direction.y))?;
                    writer.write_code_pair(&CodePair::new_f64(230, ent.extrusion_direction.z))?;
                }
            },
            &EntityType::DgnUnderlay(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbUnderlayReference")))?;
                writer.write_code_pair(&CodePair::new_string(340, &ent.__object_handle.as_string()))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.insertion_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.insertion_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.insertion_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.x_scale))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.y_scale))?;
                writer.write_code_pair(&CodePair::new_f64(43, ent.z_scale))?;
                writer.write_code_pair(&CodePair::new_f64(50, ent.rotation_angle))?;
                writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                writer.write_code_pair(&CodePair::new_i16(280, ent.flags as i16))?;
                writer.write_code_pair(&CodePair::new_i16(281, ent.contrast))?;
                writer.write_code_pair(&CodePair::new_i16(282, ent.fade))?;
                for item in &ent.points {
                    writer.write_code_pair(&CodePair::new_f64(11, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(12, item.y))?;
                }
            },
            &EntityType::DwfUnderlay(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbUnderlayReference")))?;
                writer.write_code_pair(&CodePair::new_string(340, &ent.__object_handle.as_string()))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.insertion_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.insertion_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.insertion_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.x_scale))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.y_scale))?;
                writer.write_code_pair(&CodePair::new_f64(43, ent.z_scale))?;
                writer.write_code_pair(&CodePair::new_f64(50, ent.rotation_angle))?;
                writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                writer.write_code_pair(&CodePair::new_i16(280, ent.flags as i16))?;
                writer.write_code_pair(&CodePair::new_i16(281, ent.contrast))?;
                writer.write_code_pair(&CodePair::new_i16(282, ent.fade))?;
                for item in &ent.points {
                    writer.write_code_pair(&CodePair::new_f64(11, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(12, item.y))?;
                }
            },
            &EntityType::PdfUnderlay(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbUnderlayReference")))?;
                writer.write_code_pair(&CodePair::new_string(340, &ent.__object_handle.as_string()))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.insertion_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.insertion_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.insertion_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(41, ent.x_scale))?;
                writer.write_code_pair(&CodePair::new_f64(42, ent.y_scale))?;
                writer.write_code_pair(&CodePair::new_f64(43, ent.z_scale))?;
                writer.write_code_pair(&CodePair::new_f64(50, ent.rotation_angle))?;
                writer.write_code_pair(&CodePair::new_f64(210, ent.normal.x))?;
                writer.write_code_pair(&CodePair::new_f64(220, ent.normal.y))?;
                writer.write_code_pair(&CodePair::new_f64(230, ent.normal.z))?;
                writer.write_code_pair(&CodePair::new_i16(280, ent.flags as i16))?;
                writer.write_code_pair(&CodePair::new_i16(281, ent.contrast))?;
                writer.write_code_pair(&CodePair::new_i16(282, ent.fade))?;
                for item in &ent.points {
                    writer.write_code_pair(&CodePair::new_f64(11, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(12, item.y))?;
                }
            },
            &EntityType::Vertex(_) => { panic!("this case should have been covered in a custom writer"); },
            &EntityType::Wipeout(ref ent) => {
                writer.write_code_pair(&CodePair::new_string(100, &String::from("AcDbRasterImage")))?;
                writer.write_code_pair(&CodePair::new_i32(90, ent.class_version))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.location.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.location.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.location.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.u_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.u_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.u_vector.z))?;
                writer.write_code_pair(&CodePair::new_f64(12, ent.v_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(22, ent.v_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(32, ent.v_vector.z))?;
                writer.write_code_pair(&CodePair::new_f64(13, ent.image_size.x))?;
                writer.write_code_pair(&CodePair::new_f64(23, ent.image_size.y))?;
                writer.write_code_pair(&CodePair::new_string(340, &ent.image_def_reference))?;
                writer.write_code_pair(&CodePair::new_i16(70, ent.display_options_flags as i16))?;
                writer.write_code_pair(&CodePair::new_i16(280, as_i16(ent.use_clipping)))?;
                writer.write_code_pair(&CodePair::new_i16(281, ent.brightness))?;
                writer.write_code_pair(&CodePair::new_i16(282, ent.contrast))?;
                writer.write_code_pair(&CodePair::new_i16(283, ent.fade))?;
                writer.write_code_pair(&CodePair::new_string(360, &ent.image_def_reactor_reference))?;
                writer.write_code_pair(&CodePair::new_i16(71, ent.clipping_type as i16))?;
                writer.write_code_pair(&CodePair::new_i32(91, ent.clipping_vertices.len() as i32))?;
                for item in &ent.clipping_vertices {
                    writer.write_code_pair(&CodePair::new_f64(14, item.x))?;
                    writer.write_code_pair(&CodePair::new_f64(24, item.y))?;
                }
                if version >= AcadVersion::R2010 {
                    writer.write_code_pair(&CodePair::new_bool(290, ent.is_inside_clipping))?;
                }
            },
            &EntityType::XLine(ref ent) => {
                writer.write_code_pair(&CodePair::new_str(100, "AcDbXline"))?;
                writer.write_code_pair(&CodePair::new_f64(10, ent.first_point.x))?;
                writer.write_code_pair(&CodePair::new_f64(20, ent.first_point.y))?;
                writer.write_code_pair(&CodePair::new_f64(30, ent.first_point.z))?;
                writer.write_code_pair(&CodePair::new_f64(11, ent.unit_direction_vector.x))?;
                writer.write_code_pair(&CodePair::new_f64(21, ent.unit_direction_vector.y))?;
                writer.write_code_pair(&CodePair::new_f64(31, ent.unit_direction_vector.z))?;
            },
        }

        Ok(())
    }
}
