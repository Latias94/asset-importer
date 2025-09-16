//! Complete Model Loading Demo with glow + winit + asset-importer
//!
//! This example demonstrates a complete 3D model loading and rendering pipeline
//! using glow (OpenGL), winit 0.30, and asset-importer (Assimp bindings).
//!
//! Based on LearnOpenGL Model Loading tutorial:
//! https://learnopengl.com/Model-Loading/Assimp
//! https://learnopengl.com/Model-Loading/Mesh
//! https://learnopengl.com/Model-Loading/Model
//!
//! Usage: cargo run --example model_loading_demo -- <model_file>

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use asset_importer::{material::TextureType, postprocess::PostProcessSteps, Importer};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use glow::*;
use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SwapInterval, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    raw_window_handle::HasWindowHandle,
    window::{WindowAttributes, WindowId},
};

// Camera movement directions
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CameraMovement {
    Forward,
    Backward,
    Left,
    Right,
}

// Vertex shader source
const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec2 aTexCoords;

out vec3 FragPos;
out vec3 Normal;
out vec2 TexCoords;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main()
{
    FragPos = vec3(model * vec4(aPos, 1.0));
    Normal = mat3(transpose(inverse(model))) * aNormal;
    TexCoords = aTexCoords;

    gl_Position = projection * view * vec4(FragPos, 1.0);
}
"#;

// Fragment shader source
const FRAGMENT_SHADER_SOURCE: &str = r#"
#version 330 core
out vec4 FragColor;

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoords;

uniform vec3 lightPos;
uniform vec3 lightColor;
uniform vec3 viewPos;
uniform vec3 objectColor;

uniform sampler2D texture_diffuse1;
uniform bool hasTexture;

void main()
{
    // 环境光
    float ambientStrength = 0.3;
    vec3 ambient = ambientStrength * lightColor;

    // 漫反射光
    vec3 norm = normalize(Normal);
    vec3 lightDir = normalize(lightPos - FragPos);
    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = diff * lightColor;

    // 镜面反射光
    float specularStrength = 0.5;
    vec3 viewDir = normalize(viewPos - FragPos);
    vec3 reflectDir = reflect(-lightDir, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32);
    vec3 specular = specularStrength * spec * lightColor;

    vec3 result;
    if (hasTexture) {
        vec3 textureColor = texture(texture_diffuse1, TexCoords).rgb;
        result = (ambient + diffuse + specular) * textureColor;
    } else {
        result = (ambient + diffuse + specular) * objectColor;
    }

    FragColor = vec4(result, 1.0);
}
"#;

/// Vertex structure matching LearnOpenGL tutorial
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}

/// Texture information
#[derive(Debug, Clone)]
struct TextureInfo {
    id: Option<glow::NativeTexture>,
    texture_type: String,
    path: String,
}

/// Mesh containing vertex data and OpenGL objects
struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    textures: Vec<TextureInfo>,
    vao: Option<glow::NativeVertexArray>,
    vbo: Option<glow::NativeBuffer>,
    ebo: Option<glow::NativeBuffer>,
}

impl Mesh {
    fn new(
        gl: &glow::Context,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        textures: Vec<TextureInfo>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut mesh = Self {
            vertices,
            indices,
            textures,
            vao: None,
            vbo: None,
            ebo: None,
        };

        mesh.setup_mesh(gl)?;
        Ok(mesh)
    }

    fn setup_mesh(&mut self, gl: &glow::Context) -> Result<(), Box<dyn Error>> {
        unsafe {
            // Generate buffers
            self.vao = Some(gl.create_vertex_array()?);
            self.vbo = Some(gl.create_buffer()?);
            self.ebo = Some(gl.create_buffer()?);

            gl.bind_vertex_array(self.vao);

            // Load vertex data
            gl.bind_buffer(glow::ARRAY_BUFFER, self.vbo);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&self.vertices),
                glow::STATIC_DRAW,
            );

            // Load index data
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&self.indices),
                glow::STATIC_DRAW,
            );

            // Set vertex attributes
            let stride = std::mem::size_of::<Vertex>() as i32;

            // Position attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

            // Normal attribute
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);

            // Texture coordinate attribute
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 24);

            gl.bind_vertex_array(None);
        }
        Ok(())
    }

    fn draw(&self, gl: &glow::Context, shader_program: Option<glow::NativeProgram>) {
        unsafe {
            // Bind textures
            let mut diffuse_nr = 1;
            let mut specular_nr = 1;

            for (i, texture) in self.textures.iter().enumerate() {
                gl.active_texture(glow::TEXTURE0 + i as u32);

                let number = if texture.texture_type == "texture_diffuse" {
                    let num = diffuse_nr;
                    diffuse_nr += 1;
                    num
                } else if texture.texture_type == "texture_specular" {
                    let num = specular_nr;
                    specular_nr += 1;
                    num
                } else {
                    1
                };

                let uniform_name = format!("material.{}{}", texture.texture_type, number);
                let location = gl.get_uniform_location(shader_program.unwrap(), &uniform_name);
                if let Some(loc) = location {
                    gl.uniform_1_i32(Some(&loc), i as i32);
                }

                gl.bind_texture(glow::TEXTURE_2D, texture.id);
            }

            gl.active_texture(glow::TEXTURE0);

            // Draw mesh
            gl.bind_vertex_array(self.vao);
            gl.draw_elements(
                glow::TRIANGLES,
                self.indices.len() as i32,
                glow::UNSIGNED_INT,
                0,
            );
            gl.bind_vertex_array(None);
        }
    }
}

/// Model containing multiple meshes
struct Model {
    meshes: Vec<Mesh>,
    textures_loaded: Vec<TextureInfo>,
    directory: String,
}

impl Model {
    fn new(gl: &glow::Context, path: &str) -> Result<Self, Box<dyn Error>> {
        let mut model = Self {
            meshes: Vec::new(),
            textures_loaded: Vec::new(),
            directory: Path::new(path)
                .parent()
                .unwrap_or(Path::new(""))
                .to_string_lossy()
                .to_string(),
        };

        model.load_model(gl, path)?;
        Ok(model)
    }

    fn new_without_gl(path: &str) -> Result<Self, Box<dyn Error>> {
        let mut model = Self {
            meshes: Vec::new(),
            textures_loaded: Vec::new(),
            directory: std::path::Path::new(path)
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""))
                .to_string_lossy()
                .to_string(),
        };

        model.load_model_without_gl(path)?;
        Ok(model)
    }

    fn load_model(&mut self, gl: &glow::Context, path: &str) -> Result<(), Box<dyn Error>> {
        println!("Loading model: {}", path);

        let importer = Importer::new();
        let scene = importer
            .read_file(path)
            .with_post_process(
                PostProcessSteps::TRIANGULATE
                    | PostProcessSteps::FLIP_UVS
                    | PostProcessSteps::GEN_SMOOTH_NORMALS
                    | PostProcessSteps::CALC_TANGENT_SPACE,
            )
            .import_file(path)?;

        println!("Scene loaded successfully!");
        println!("  Meshes: {}", scene.num_meshes());
        println!("  Materials: {}", scene.num_materials());
        println!("  Textures: {}", scene.num_textures());

        if let Some(root_node) = scene.root_node() {
            self.process_node(gl, &root_node, &scene)?;
        }

        println!(
            "Model processing complete. Total meshes: {}",
            self.meshes.len()
        );
        Ok(())
    }

    fn load_model_without_gl(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        println!("Loading model: {}", path);

        let importer = Importer::new();
        let scene = importer
            .read_file(path)
            .with_post_process(
                PostProcessSteps::TRIANGULATE
                    | PostProcessSteps::FLIP_UVS
                    | PostProcessSteps::GEN_SMOOTH_NORMALS
                    | PostProcessSteps::CALC_TANGENT_SPACE,
            )
            .import_file(path)?;

        println!("Scene loaded successfully!");
        println!("  Meshes: {}", scene.num_meshes());
        println!("  Materials: {}", scene.num_materials());
        println!("  Textures: {}", scene.num_textures());

        if let Some(root_node) = scene.root_node() {
            self.process_node_without_gl(&root_node, &scene)?;
        }

        println!(
            "Model processing complete. Total meshes: {}",
            self.meshes.len()
        );
        Ok(())
    }

    fn process_node(
        &mut self,
        gl: &glow::Context,
        node: &asset_importer::node::Node,
        scene: &asset_importer::scene::Scene,
    ) -> Result<(), Box<dyn Error>> {
        // Process all meshes in this node
        for mesh_index in node.mesh_indices() {
            if let Some(mesh) = scene.mesh(mesh_index) {
                let processed_mesh = self.process_mesh(gl, &mesh, scene)?;
                self.meshes.push(processed_mesh);
            }
        }

        // Process all child nodes
        for child in node.children() {
            self.process_node(gl, &child, scene)?;
        }

        Ok(())
    }

    fn process_node_without_gl(
        &mut self,
        node: &asset_importer::node::Node,
        scene: &asset_importer::scene::Scene,
    ) -> Result<(), Box<dyn Error>> {
        // Process all meshes in this node
        for mesh_index in node.mesh_indices() {
            if let Some(mesh) = scene.mesh(mesh_index) {
                let processed_mesh = self.process_mesh_without_gl(&mesh, scene)?;
                self.meshes.push(processed_mesh);
            }
        }

        // Process all child nodes
        for child in node.children() {
            self.process_node_without_gl(&child, scene)?;
        }

        Ok(())
    }

    fn process_mesh(
        &mut self,
        gl: &glow::Context,
        mesh: &asset_importer::mesh::Mesh,
        scene: &asset_importer::scene::Scene,
    ) -> Result<Mesh, Box<dyn Error>> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut textures = Vec::new();

        // Process vertices
        let positions = mesh.vertices();
        let normals = mesh.normals().unwrap_or_default();
        let tex_coords = mesh.texture_coords(0).unwrap_or_default();

        for i in 0..positions.len() {
            let vertex = Vertex {
                position: [positions[i].x, positions[i].y, positions[i].z],
                normal: if i < normals.len() {
                    [normals[i].x, normals[i].y, normals[i].z]
                } else {
                    [0.0, 1.0, 0.0]
                },
                tex_coords: if i < tex_coords.len() {
                    [tex_coords[i].x, tex_coords[i].y]
                } else {
                    [0.0, 0.0]
                },
            };
            vertices.push(vertex);
        }

        // Process indices
        for face in mesh.faces() {
            for &index in face.indices() {
                indices.push(index);
            }
        }

        // Process material
        if let Some(material) = scene.material(mesh.material_index()) {
            // Load diffuse textures
            let mut diffuse_maps = self.load_material_textures(
                gl,
                &material,
                TextureType::Diffuse,
                "texture_diffuse",
            )?;
            textures.append(&mut diffuse_maps);

            // Load specular textures
            let mut specular_maps = self.load_material_textures(
                gl,
                &material,
                TextureType::Specular,
                "texture_specular",
            )?;
            textures.append(&mut specular_maps);
        }

        Mesh::new(gl, vertices, indices, textures)
    }

    fn process_mesh_without_gl(
        &mut self,
        mesh: &asset_importer::mesh::Mesh,
        _scene: &asset_importer::scene::Scene,
    ) -> Result<Mesh, Box<dyn Error>> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let textures = Vec::new(); // No texture loading without GL context

        // Process vertices
        let positions = mesh.vertices();
        let normals = mesh.normals().unwrap_or_default();
        let tex_coords = mesh.texture_coords(0).unwrap_or_default();

        for i in 0..positions.len() {
            let vertex = Vertex {
                position: [positions[i].x, positions[i].y, positions[i].z],
                normal: if i < normals.len() {
                    [normals[i].x, normals[i].y, normals[i].z]
                } else {
                    [0.0, 1.0, 0.0]
                },
                tex_coords: if i < tex_coords.len() {
                    [tex_coords[i].x, tex_coords[i].y]
                } else {
                    [0.0, 0.0]
                },
            };
            vertices.push(vertex);
        }

        // Process indices
        for face in mesh.faces() {
            for &index in face.indices() {
                indices.push(index);
            }
        }

        // Create mesh without OpenGL setup
        Ok(Mesh {
            vertices,
            indices,
            textures,
            vao: None,
            vbo: None,
            ebo: None,
        })
    }

    fn load_material_textures(
        &mut self,
        gl: &glow::Context,
        material: &asset_importer::material::Material,
        tex_type: TextureType,
        type_name: &str,
    ) -> Result<Vec<TextureInfo>, Box<dyn Error>> {
        let mut textures = Vec::new();
        let texture_count = material.texture_count(tex_type);

        for i in 0..texture_count {
            if let Some(texture_info) = material.texture(tex_type, i) {
                let texture_path = texture_info.path.clone();

                // Check if texture was already loaded
                if let Some(loaded_texture) =
                    self.textures_loaded.iter().find(|t| t.path == texture_path)
                {
                    textures.push(loaded_texture.clone());
                    continue;
                }

                // Load new texture
                let texture_id = self.load_texture_from_file(gl, &texture_path)?;
                let texture = TextureInfo {
                    id: Some(texture_id),
                    texture_type: type_name.to_string(),
                    path: texture_path.clone(),
                };

                textures.push(texture.clone());
                self.textures_loaded.push(texture);
            }
        }

        Ok(textures)
    }

    fn load_texture_from_file(
        &self,
        gl: &glow::Context,
        path: &str,
    ) -> Result<glow::NativeTexture, Box<dyn Error>> {
        let full_path = if self.directory.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.directory, path)
        };

        println!("Loading texture: {}", full_path);

        // Try to load the image
        let img = match image::open(&full_path) {
            Ok(img) => img.to_rgba8(),
            Err(_) => {
                println!("Failed to load texture: {}, using default", full_path);
                // Create a simple default texture (white)
                image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
            }
        };

        let (width, height) = img.dimensions();
        let data = img.into_raw();

        unsafe {
            let texture = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&data),
            );

            gl.generate_mipmap(glow::TEXTURE_2D);

            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );

            Ok(texture)
        }
    }

    fn draw(&self, gl: &glow::Context, shader_program: Option<glow::NativeProgram>) {
        for mesh in &self.meshes {
            mesh.draw(gl, shader_program);
        }
    }
}

/// Camera for FPS-style movement (based on learn_opengl_rs)
pub struct Camera {
    // camera attributes
    pub position: Vec3,
    pub front: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub world_up: Vec3,
    // euler angles
    pub yaw: f32,
    pub pitch: f32,
    // camera options
    pub movement_speed: f32,
    pub mouse_sensitivity: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn new_with_position(position: Vec3) -> Self {
        let mut camera = Self {
            position,
            front: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::ZERO,
            right: Vec3::ZERO,
            world_up: Vec3::Y,
            yaw: -90.0,
            pitch: 0.0,
            movement_speed: 2.5,
            mouse_sensitivity: 0.1,
            zoom: 45.0,
        };
        camera.update_camera_vectors();
        camera
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, self.up)
    }

    pub fn process_keyboard(&mut self, direction: CameraMovement, delta_time: f32) {
        let velocity = self.movement_speed * delta_time;
        match direction {
            CameraMovement::Forward => self.position += self.front * velocity,
            CameraMovement::Backward => self.position -= self.front * velocity,
            CameraMovement::Left => self.position -= self.right * velocity,
            CameraMovement::Right => self.position += self.right * velocity,
        }
    }

    pub fn process_mouse_movement(
        &mut self,
        mut xoffset: f32,
        mut yoffset: f32,
        constrain_pitch: bool,
    ) {
        xoffset *= self.mouse_sensitivity;
        yoffset *= self.mouse_sensitivity;

        self.yaw += xoffset;
        self.pitch += yoffset;

        if constrain_pitch {
            if self.pitch > 89.0 {
                self.pitch = 89.0;
            }
            if self.pitch < -89.0 {
                self.pitch = -89.0;
            }
        }

        self.update_camera_vectors();
    }

    pub fn process_mouse_scroll(&mut self, yoffset: f32) {
        self.zoom -= yoffset;
        if self.zoom < 1.0 {
            self.zoom = 1.0;
        }
        if self.zoom > 45.0 {
            self.zoom = 45.0;
        }
    }

    fn update_camera_vectors(&mut self) {
        let front = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        );
        self.front = front.normalize();
        self.right = self.front.cross(self.world_up).normalize();
        self.up = self.right.cross(self.front).normalize();
    }
}

/// Shader utilities
struct Shader {
    program: Option<glow::NativeProgram>,
}

impl Shader {
    fn new(
        gl: &glow::Context,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<Self, Box<dyn Error>> {
        unsafe {
            let vertex_shader = gl.create_shader(glow::VERTEX_SHADER)?;
            gl.shader_source(vertex_shader, vertex_source);
            gl.compile_shader(vertex_shader);

            if !gl.get_shader_compile_status(vertex_shader) {
                let error = gl.get_shader_info_log(vertex_shader);
                gl.delete_shader(vertex_shader);
                return Err(format!("Vertex shader compilation failed: {}", error).into());
            }

            let fragment_shader = gl.create_shader(glow::FRAGMENT_SHADER)?;
            gl.shader_source(fragment_shader, fragment_source);
            gl.compile_shader(fragment_shader);

            if !gl.get_shader_compile_status(fragment_shader) {
                let error = gl.get_shader_info_log(fragment_shader);
                gl.delete_shader(vertex_shader);
                gl.delete_shader(fragment_shader);
                return Err(format!("Fragment shader compilation failed: {}", error).into());
            }

            let program = gl.create_program()?;
            gl.attach_shader(program, vertex_shader);
            gl.attach_shader(program, fragment_shader);
            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                let error = gl.get_program_info_log(program);
                gl.delete_shader(vertex_shader);
                gl.delete_shader(fragment_shader);
                gl.delete_program(program);
                return Err(format!("Shader program linking failed: {}", error).into());
            }

            gl.delete_shader(vertex_shader);
            gl.delete_shader(fragment_shader);

            Ok(Self {
                program: Some(program),
            })
        }
    }

    fn use_program(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(self.program);
        }
    }

    fn set_mat4(&self, gl: &glow::Context, name: &str, mat: &Mat4) {
        unsafe {
            let location = gl.get_uniform_location(self.program.unwrap(), name);
            if let Some(loc) = location {
                gl.uniform_matrix_4_f32_slice(Some(&loc), false, &mat.to_cols_array());
            }
        }
    }

    fn set_vec3(&self, gl: &glow::Context, name: &str, vec: Vec3) {
        unsafe {
            let location = gl.get_uniform_location(self.program.unwrap(), name);
            if let Some(loc) = location {
                gl.uniform_3_f32(Some(&loc), vec.x, vec.y, vec.z);
            }
        }
    }

    fn set_float(&self, gl: &glow::Context, name: &str, value: f32) {
        unsafe {
            let location = gl.get_uniform_location(self.program.unwrap(), name);
            if let Some(loc) = location {
                gl.uniform_1_f32(Some(&loc), value);
            }
        }
    }

    fn set_bool(&self, gl: &glow::Context, name: &str, value: bool) {
        unsafe {
            let location = gl.get_uniform_location(self.program.unwrap(), name);
            if let Some(loc) = location {
                gl.uniform_1_i32(Some(&loc), if value { 1 } else { 0 });
            }
        }
    }
}

/// OpenGL context and surface
struct GlContext {
    gl: glow::Context,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
}

/// Main application state
struct App {
    window: winit::window::Window,
    gl: glow::Context,
    gl_surface: Surface<WindowSurface>,
    gl_context: PossiblyCurrentContext,
    shader: Shader,
    model: Model,
    camera: Camera,
    last_frame: std::time::Instant,
    first_mouse: bool,
    last_x: f32,
    last_y: f32,
    keys_pressed: HashMap<KeyCode, bool>,
}

impl App {
    fn new_with_context(
        window: winit::window::Window,
        gl: glow::Context,
        gl_surface: Surface<WindowSurface>,
        gl_context: PossiblyCurrentContext,
        shader: Shader,
        model: Model,
    ) -> Self {
        Self {
            window,
            gl,
            gl_surface,
            gl_context,
            shader,
            model,
            camera: Camera::new_with_position(Vec3::new(0.0, 0.0, 8.0)),
            last_frame: std::time::Instant::now(),
            first_mouse: true,
            last_x: 400.0,
            last_y: 300.0,
            keys_pressed: HashMap::new(),
        }
    }

    fn process_input(&mut self, delta_time: f32) {
        if *self.keys_pressed.get(&KeyCode::KeyW).unwrap_or(&false) {
            self.camera
                .process_keyboard(CameraMovement::Forward, delta_time);
        }
        if *self.keys_pressed.get(&KeyCode::KeyS).unwrap_or(&false) {
            self.camera
                .process_keyboard(CameraMovement::Backward, delta_time);
        }
        if *self.keys_pressed.get(&KeyCode::KeyA).unwrap_or(&false) {
            self.camera
                .process_keyboard(CameraMovement::Left, delta_time);
        }
        if *self.keys_pressed.get(&KeyCode::KeyD).unwrap_or(&false) {
            self.camera
                .process_keyboard(CameraMovement::Right, delta_time);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Context is already initialized in main
        println!("Application resumed");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                unsafe {
                    self.gl
                        .viewport(0, 0, size.width as i32, size.height as i32);
                }
                self.gl_surface.resize(
                    &self.gl_context,
                    std::num::NonZeroU32::new(size.width.max(1)).unwrap(),
                    std::num::NonZeroU32::new(size.height.max(1)).unwrap(),
                );
                self.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.render();
                self.window.pre_present_notify();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys_pressed.insert(keycode, true);
                            if keycode == KeyCode::Escape {
                                event_loop.exit();
                            }
                        }
                        ElementState::Released => {
                            self.keys_pressed.insert(keycode, false);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32;
                let y = position.y as f32;

                if self.first_mouse {
                    self.last_x = x;
                    self.last_y = y;
                    self.first_mouse = false;
                }

                let xoffset = x - self.last_x;
                let yoffset = self.last_y - y; // reversed since y-coordinates go from bottom to top

                self.last_x = x;
                self.last_y = y;

                self.camera.process_mouse_movement(xoffset, yoffset, true);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let winit::event::MouseScrollDelta::LineDelta(_, y) = delta {
                    self.camera.process_mouse_scroll(y);
                }
            }
            _ => {}
        }
    }
}

impl App {
    fn render(&mut self) {
        let current_frame = std::time::Instant::now();
        let delta_time = current_frame.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = current_frame;

        self.process_input(delta_time);

        unsafe {
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        self.shader.use_program(&self.gl);

        // Set up matrices
        let size = self.window.inner_size();
        let projection = Mat4::perspective_rh_gl(
            self.camera.zoom.to_radians(),
            size.width as f32 / size.height as f32,
            0.1,
            100.0,
        );
        let view = self.camera.view_matrix();
        let model_matrix = Mat4::IDENTITY; // 使用原始大小

        self.shader.set_mat4(&self.gl, "projection", &projection);
        self.shader.set_mat4(&self.gl, "view", &view);
        self.shader.set_mat4(&self.gl, "model", &model_matrix);

        // Set up lighting
        self.shader
            .set_vec3(&self.gl, "lightPos", Vec3::new(2.0, 2.0, 2.0));
        self.shader
            .set_vec3(&self.gl, "lightColor", Vec3::new(1.0, 1.0, 1.0));
        self.shader
            .set_vec3(&self.gl, "viewPos", self.camera.position);
        self.shader
            .set_vec3(&self.gl, "objectColor", Vec3::new(0.8, 0.6, 0.4)); // 橙色

        // 检测是否有纹理
        let has_texture = !self.model.textures_loaded.is_empty();
        self.shader.set_bool(&self.gl, "hasTexture", has_texture);

        // Draw model
        self.model.draw(&self.gl, self.shader.program);

        // Swap buffers
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();

        self.window.request_redraw();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Get model path from command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <model_file>", args[0]);
        eprintln!("Example: {} models/backpack/backpack.obj", args[0]);
        std::process::exit(1);
    }

    let model_path = args[1].clone();

    // Check if model file exists
    if !std::path::Path::new(&model_path).exists() {
        eprintln!("Error: Model file '{}' does not exist", model_path);
        std::process::exit(1);
    }

    println!("🎮 Model Loading Demo");
    println!("📁 Loading model: {}", model_path);
    println!("🎯 Controls:");
    println!("   WASD - Move camera");
    println!("   Mouse - Look around");
    println!("   Mouse wheel - Zoom");
    println!("   ESC - Exit");

    // Create event loop first
    let event_loop = EventLoop::new()?;

    // Create window and OpenGL context
    let window_attributes = WindowAttributes::default()
        .with_title("Model Loading Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));

    let template = ConfigTemplateBuilder::new()
        .with_depth_size(24)
        .with_stencil_size(8);

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

    let (window, gl_config) = display_builder.build(&event_loop, template, |configs| {
        configs
            .reduce(|accum, config| {
                if config.num_samples() > accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
            .unwrap()
    })?;

    let window = window.unwrap();
    let raw_window_handle = window.window_handle().ok().map(|h| h.as_raw());

    let gl_display = gl_config.display();
    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(glutin::context::Version {
            major: 3,
            minor: 3,
        })))
        .build(raw_window_handle);

    let not_current_gl_context =
        unsafe { gl_display.create_context(&gl_config, &context_attributes)? };

    let attrs = window.build_surface_attributes(Default::default())?;
    let gl_surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };

    let gl_context = not_current_gl_context.make_current(&gl_surface)?;

    let gl =
        unsafe { glow::Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s)) };

    println!("OpenGL version: {}", unsafe {
        gl.get_parameter_string(glow::VERSION)
    });
    println!("OpenGL renderer: {}", unsafe {
        gl.get_parameter_string(glow::RENDERER)
    });

    // Initialize OpenGL settings
    unsafe {
        gl.enable(glow::DEPTH_TEST);
        gl.clear_color(0.1, 0.1, 0.1, 1.0);
    }

    // Load model
    println!("📦 Loading model: {}", model_path);
    let model = Model::new(&gl, &model_path)?;

    println!("✅ Model loaded successfully!");
    println!("📊 Model Statistics:");
    println!("   - Meshes: {}", model.meshes.len());
    println!("   - Textures: {}", model.textures_loaded.len());

    // Create shader
    let shader = Shader::new(&gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE)?;

    // Create app with initialized context
    let mut app = App::new_with_context(window, gl, gl_surface, gl_context, shader, model);

    event_loop.run_app(&mut app)?;

    Ok(())
}
