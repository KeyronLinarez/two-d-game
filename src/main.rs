use std::borrow::Cow;

mod input;
mod game_state;
use rand::Rng;
use bytemuck::{Pod, Zeroable};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use glyphon::{
    Attrs, Buffer, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer,
};
use wgpu::{
    CompositeAlphaMode, MultisampleState, 
};
use game_state::GameState;

pub const WINDOW_WIDTH: f32 = 1024.0;
pub const WINDOW_HEIGHT: f32 = 768.0;
pub const SPRITE_SIZE: f32 = 64.0;




// In WGPU, we define an async function whose operation can be suspended and resumed.
// This is because on web, we can't take over the main event loop and must leave it to
// the browser.  On desktop, we'll just be running this function to completion.
async fn run(event_loop: EventLoop<()>, window: Window) {
    // state of game at any time
    let mut gs = game_state::init_game_state();


    // sprite struct
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
    #[derive(Debug)]
    struct GPUSprite {
        to_region: [f32;4],
        from_region: [f32;4],
    }

    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
    struct bullet {
        x: f32,
        y: f32,
    }

    // camera struct
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
    struct GPUCamera {
        screen_pos: [f32;2],
        screen_size: [f32;2]
    }

    // camera stuff
    let camera = GPUCamera {
        screen_pos: [0.0, 0.0],
        // Consider using config.width and config.height instead,
        // it's up to you whether you want the window size to change what's visible in the game
        // or scale it up and down
        //              x       y
        screen_size: [WINDOW_WIDTH, WINDOW_HEIGHT],
    };

    // VECTOR OF POS OF OUR SPRITES
    // MATH CORDS = 0,0 == BOTTOM LEFT
    // SCREEN CORDS = 0,0 == TOP LEFT

    // let mut sprites:Vec<GPUSprite> = vec![];
    // let mut i = 0;
    // while i < 4{
    //     sprites.push(GPUSprite {
    //         to_region: [384.0 + ((64*i) as f32), 512.0, SPRITE_SIZE, SPRITE_SIZE],
    //         from_region: [0.25, 0.0, 0.25, 0.1],
    //     });
    //     i += 1;
    // }
    let mut rng = rand::thread_rng();
    // number of max dropped per row * 12 is the maximum number of sprites needed for the game.
    // MAKE BUFFER BIGGER FOR SPACE GAME
    let mut sprites:Vec<_> = (0..gs.drop_sprite_blocks*36).map(|_| GPUSprite{
        to_region: 
            [WINDOW_WIDTH/2.0,
            WINDOW_HEIGHT,
            0.0, // generate width and height to be 0 so that you can adjust later, but are now invisible
            0.0], 
        from_region:[
            0.25, // + rng.gen_range(0..2) as f32*0.25, // each row needs to be the same color, so all random doesn't do anything
            0.0, // + rng.gen_range(0..10) as f32*0.1,
            0.25,
            0.1],
    }).collect();
    
    
    use std::path::Path;
    let img = image::open(Path::new("content/block-sprites.png")).expect("Should be a valid image at path content/block-sprites.png'");
    let img = img.to_rgba8();
    let size = window.inner_size();
    let instance = wgpu::Instance::default();
    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        // This operation can take some time, so we await the result. We can only await like this
        // in an async function.
        .await
        // And it can fail, so we panic with an error message if we can't get a GPU.
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue.  A logical device is like a connection to a GPU, and
    // we'll be issuing instructions to the GPU over the command queue.

    let (device, queue) = adapter
    .request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            // Bump up the limits to require the availability of storage buffers.
            limits: wgpu::Limits::downlevel_defaults()
                .using_resolution(adapter.limits()),
        },
        None,
    )
    .await
    .expect("Failed to create device");



    let buffer_camera = device.create_buffer(&wgpu::BufferDescriptor{
        label: None,
        size: bytemuck::bytes_of(&camera).len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false
    });
    let buffer_sprite = device.create_buffer(&wgpu::BufferDescriptor{
        label: None,
        size: (bytemuck::cast_slice::<_,u8>(&sprites).len()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false
    });
    // have sprites be a fixed length, track how far you have gone into sprites,
    // when disappeared, make width and height = 0



        let (img_w, img_h) = img.dimensions();
        // How big is the texture in GPU memory?
        let size = wgpu::Extent3d {
            width: img_w,
            height: img_h,
            depth_or_array_layers: 1,
        };
        // Let's make a texture now
        let texture = device.create_texture(
            // Parameters for the texture
            &wgpu::TextureDescriptor {
                // An optional label
                label: Some("47 image"),
                // Its dimensions. This line is equivalent to size:size
                size,
                // Number of mipmapping levels (to show different pictures at different distances)
                mip_level_count: 1,
                // Number of samples per pixel in the texture. It'll be one for our whole class.
                sample_count: 1,
                // Is it a 1D, 2D, or 3D texture?
                dimension: wgpu::TextureDimension::D2,
                // 8 bits per component, four components per pixel, unsigned, normalized in 0..255, SRGB
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                // This texture will be bound for shaders and have stuff copied to it
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                // What formats are allowed as views on this texture besides the native format
                view_formats: &[],
            }
        );
        // Now that we have a texture, we need to copy its data to the GPU:
        queue.write_texture(
            // A description of where to write the image data.
            // We'll use this helper to say "the whole texture"
            texture.as_image_copy(),
            // The image data to write
            &img,
            // What portion of the image data to copy from the CPU
            wgpu::ImageDataLayout {
                // Where in img do the bytes to copy start?
                offset: 0,
                // How many bytes in each row of the image?
                bytes_per_row: Some(4*img_w),
                // We could pass None here and it would be alright,
                // since we're only uploading one image
                rows_per_image: Some(img_h),
            },
            // What portion of the texture we're writing into
            size
        );

        // ADD DATA INTO THE BUFFERS!!!!
        queue.write_buffer(&buffer_camera, 0, bytemuck::bytes_of(&camera));
        queue.write_buffer(&buffer_sprite, 0, bytemuck::cast_slice(&sprites));
        //queue.write_buffer(&buffer_sprite, 0, bytemuck::cast_slice(&sprites));


    // The swapchain is how we obtain images from the surface we're drawing onto.
    // This is so we can draw onto one image while a different one is being presentedto the user on-screen.
    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&device, &config);


    // Set up text renderer
    let mut font_system = FontSystem::new();
    let mut cache = SwashCache::new();
    let mut atlas = TextAtlas::new(&device, &queue, swapchain_format);
    let mut text_renderer =
        TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(30.0, 42.0));


    let physical_width = (size.width as f64 * window.scale_factor()) as f32;
    let physical_height = (size.height as f64 * window.scale_factor()) as f32;


    buffer.set_size(&mut font_system, WINDOW_WIDTH, WINDOW_HEIGHT);
    buffer.set_text(&mut font_system, "Block Games!!!\nPress 1 for Falling Blocks\nPress 2 for Space Blocks", Attrs::new().family(Family::Serif), Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system);

    // Load the shaders from disk.  Remember, shader programs are things we compile for
    // our GPU so that it can compute vertices and colorize fragments.
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        // Cow is a "copy on write" wrapper that abstracts over owned or borrowed memory.
        // Here we just need to use it since wgpu wants "some text" to compile a shader from.
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });





    // uses helper function to load image
    // let tex_47 = load_texture("content/king.png", Some("king image"), &device, &queue)
    // .expect("Couldn't load sprite img");
    let (tex_sprite, mut img_bkgd) = load_texture("content/block-sprites.png", Some("sprite image"), &device, &queue).expect("Couldn't load sprite img");
    let view_sprite = tex_sprite.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler_sprite = device.create_sampler(&wgpu::SamplerDescriptor::default());



//////////////////////////////////

let texture_bind_group_layout =
device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    // This bind group's first entry is for the texture and the second is for the sampler.
    entries: &[
        // The texture binding
        wgpu::BindGroupLayoutEntry {
            // This matches the binding number in the shader
            binding: 0,
            // Only available in the fragment shader
            visibility: wgpu::ShaderStages::FRAGMENT,
            // It's a texture binding
            ty: wgpu::BindingType::Texture {
                // We can use it with float samplers
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                // It's being used as a 2D texture
                view_dimension: wgpu::TextureViewDimension::D2,
                // This is not a multisampled texture
                multisampled: false,
            },
            // This is not an array texture, so it has None for count
            count: None,
        },
        // The sampler binding
        wgpu::BindGroupLayoutEntry {
            // This matches the binding number in the shader
            binding: 1,
            // Only available in the fragment shader
            visibility: wgpu::ShaderStages::FRAGMENT,
            // It's a sampler
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            // No count
            count: None,
        },
    ],
});


let sprite_bind_group_layout =
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            // The camera binding
            wgpu::BindGroupLayoutEntry {
                // This matches the binding in the shader
                binding: 0,
                // Available in vertex shader
                visibility: wgpu::ShaderStages::VERTEX,
                // It's a buffer
                ty: wgpu::BindingType::Buffer {
                    // Specifically, a uniform buffer
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                // No count, not a buffer array binding
                count: None,
            },
            // The sprite buffer binding
            wgpu::BindGroupLayoutEntry {
                // This matches the binding in the shader
                binding: 1,
                // Available in vertex shader
                visibility: wgpu::ShaderStages::VERTEX,
                // It's a buffer
                ty: wgpu::BindingType::Buffer {
                    // Specifically, a storage buffer
                    ty: wgpu::BufferBindingType::Storage{read_only:true},
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                // No count, not a buffer array binding
                count: None,
            },
        ],
    });

    // BIND GROUP!!
    let sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &sprite_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_camera.as_entire_binding()
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer_sprite.as_entire_binding()
            }
        ],
    });

    let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &texture_bind_group_layout,
        entries: &[
            // One for the texture, one for the sampler
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view_sprite),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler_sprite),
            },
        ],
    });




//  gonna edit "&texture_bind_group_layout" - > 
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&sprite_bind_group_layout, &texture_bind_group_layout],
        push_constant_ranges: &[],
    });


    // Our specific "function" is going to be a draw call using our shaders. That's what we
    // set up here, calling the result a render pipeline.  I
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });
    // Definitions to control  input
    // Create a new instance of the input mod to use for the event loop
    let mut input = input::Input::default();

    // IMITATE GAME MODE SETTING IMPLEMENTATION FROM MAIN SCREEN
    // 1: EASY (start with 5, speed is 4)
    // 2: INTERMEDIATE (start with 4, speed is 6)
    // 3: HARD (start with 3, speed is 10)
    let mut game_mode: u8 = 1;


    if game_mode == 1 {
        gs.drop_sprite_blocks = 5;
        gs.speed = 4;
    }else if game_mode == 2{
        gs.drop_sprite_blocks = 4;
        gs.speed = 6;
    }else{ // game_mode == 3
        gs.drop_sprite_blocks = 3;
        gs.speed = 10;
    }

    // renders everything in the window every frame --> if we update sprite pos here, they will update
    event_loop.run(move |event, _, control_flow| {
        
        // By default, tell the windowing system that there's no more work to do
        // from the application's perspective.
        *control_flow = ControlFlow::Poll;
        // Depending on the event, we'll need to do different things.
        // There is some pretty fancy pattern matching going on here,
        // so think back to CSCI054.

        match event {
            // WindowEvent->KeyboardInput: Keyboard input!
        Event::WindowEvent {
            // Note this deeply nested pattern match
            // WindowEvent->KeyboardInput: Keyboard input!

            // Note this deeply nested pattern match
            event: WindowEvent::KeyboardInput { input: key_ev, .. },
            ..
        } => {
            input.handle_key_event(key_ev);
        },
        Event::WindowEvent {
            event: WindowEvent::MouseInput { state, button, .. },
            ..
        } => {
            input.handle_mouse_button(state, button);
        },
        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            input.handle_mouse_move(position);
        },

        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } => {
            // Reconfigure the surface with the new size
            config.width = size.width;
            config.height = size.height;
            surface.configure(&device, &config);
            // On macos the window needs to be redrawn manually after resizing
            window.request_redraw();
        },


        Event::MainEventsCleared => {
            // Reset to title screen when esc is pressed anywhere
            if input.is_key_down(winit::event::VirtualKeyCode::Escape){
                gs.screen = 0;
                sprites = (0..gs.drop_sprite_blocks*12).map(|_| GPUSprite{
                    to_region: 
                        [WINDOW_WIDTH,
                        WINDOW_HEIGHT,
                        0.0, // generate width and height to be 0 so that you can adjust later, but are now invisible
                        0.0], 
                    from_region:[
                        0.25, 
                        0.0, 
                        0.25,
                        0.1],
                }).collect();
            }
            // Check for screen number
            // Screen number: 0 = Title, 1 = Block Game, 2 = Block Setup, 3 = Black GO, 4 = Space Game, 5 = Space Setup, 6 = Space GO
            // TITLE SCREEN
            if gs.screen == 0 {

                buffer.set_text(&mut font_system, "Block Games!!!\nPress a for Falling Blocks\nPress b for Space Blocks", Attrs::new().family(Family::Serif), Shaping::Advanced);
                if input.is_key_down(winit::event::VirtualKeyCode::A){
                    gs.screen = 2;


                }else if input.is_key_down(winit::event::VirtualKeyCode::B){

                    gs = game_state::init_game_state();
                    gs.screen = 5;
                    gs.start = true;
                    //current x,y of ship
                    let bullet_speed: f32 = 5.0;
                    let cur_x: f32 = sprites[1].to_region[0];
                    let mut cur_y: f32 = sprites[1].to_region[1];


                    

                    
                    }else{

                    // FIRST SCREEN - TITLE SCREEN
                    // sprites[0].to_region = [
                    //     0.0, 
                    //     WINDOW_HEIGHT/2.0, 
                    //     WINDOW_HEIGHT/2.0, 
                    //     WINDOW_HEIGHT/2.0];
                    // sprites[0].from_region = [
                    //     0.25, 
                    //     0.1,
                    //     0.25,
                    //     0.1];
                }


            } else if gs.screen == 5 {
                        let sprites_num = sprites.len();
                        ////println!("Number of elements in the Vec: {}", sprites_num);
                        // space game
                        let text_1 = "Target practice! Hit the target for points! \nYour score: ";
                        let text = text_1.to_owned() + &gs.score.to_string();
                        buffer.set_text(&mut font_system, &text, Attrs::new().family(Family::Serif), Shaping::Advanced);

                        gs.screen = 5;
                        let mut hits: f32 = 0.0;


                        if gs.start{
                        // target sprite
                        let mut hit:bool = false;
                        let mut hit2:bool = false;
                        let mut hit3:bool = false;

                        sprites[0].to_region = [
                            gs.target_x, 
                            gs.target_y, 
                            SPRITE_SIZE, 
                            SPRITE_SIZE];
                        sprites[0].from_region = [
                            0.75, 
                            0.0,
                            0.25,
                            0.1];
                            let mut moven = gs.speed as f32;
                            if gs.direction == true{
                                moven = gs.speed as f32 * (-1.0);
                            }

                                if sprites[0].to_region[1] == WINDOW_HEIGHT - SPRITE_SIZE{
                                 gs.target_x = sprites[0].to_region[0];
                                    if gs.target_x >= 960.0 - moven{
                                        gs.direction = true;
                                    }else if gs.target_x < 0.0 + moven{
                                        gs.direction = false;
                                    }
                                    gs.target_x = gs.target_x + moven;
                                    sprites[0].to_region = [gs.target_x, WINDOW_HEIGHT - SPRITE_SIZE, SPRITE_SIZE, SPRITE_SIZE];

                            }     
                                    
                        // ship sprite VVV
                        sprites[1].to_region = [
                            gs.cur_x, 
                            gs.cur_y, 
                            SPRITE_SIZE, 
                            SPRITE_SIZE];
                        sprites[1].from_region = [
                            0.75, 
                            0.9,
                            0.25,
                            0.1];

                        // Bullet Sprites - initially invisible

                        // bullet 1
                        sprites[2].to_region = [
                            sprites[1].to_region[0], 
                            sprites[1].to_region[1], 
                            // initially invisible
                            0.0, 
                            0.0];
                        sprites[2].from_region = [
                            0.5, 
                            0.9,
                            0.25,
                            0.1];

                        // bullet 2
                        sprites[3].to_region = [
                            sprites[1].to_region[0], 
                            sprites[1].to_region[1], 
                            // initially invisible
                            0.0, 
                            0.0];
                        sprites[3].from_region = [
                            0.5, 
                            0.9,
                            0.25,
                            0.1];

                        // bullet 2
                        sprites[4].to_region = [
                            sprites[1].to_region[0], 
                            sprites[1].to_region[1], 
                            // initially invisible
                            0.0, 
                            0.0];
                        sprites[4].from_region = [
                            0.5, 
                            0.9,
                            0.25,
                            0.1];

                        //let mut bullet_sprites: Vec<GPUSprite> = vec![];

                        // checks left and right movement
                        if input.is_key_down(winit::event::VirtualKeyCode::Left){
                            ////println!("Left");
                            gs.cur_x -= 6.0;
                            sprites[1].to_region = [gs.cur_x, 0.0, SPRITE_SIZE, SPRITE_SIZE];
                            ////println!("{}", gs.cur_x);

                            if input.is_key_down(winit::event::VirtualKeyCode::Space){
                                ////println!("Space");
                                gs.bullet_count +=1;
      
                                // //new 
                                // let mut new_sprite = GPUSprite { to_region:
                                //     [gs.cur_x, 
                                //     gs.cur_y, 
                                //     // initially invisible
                                //     SPRITE_SIZE, 
                                //     SPRITE_SIZE],
                                // from_region: [
                                //     0.5, 
                                //     0.9,
                                //     0.25,
                                //     0.1]
                                // };
                                // let mut splice_num = gs.bullet_count + 3;
                                // sprites[splice_num] = new_sprite;
                                // //println!("{:#?}", new_sprite);
                                // gs.bullet_moving = true;
    
    
    
                                // // working VVVV
                                //println!("{}", gs.bullet_count.to_string());
      
                                // check if 0 bullets on screen
                                //println!("BRUHH");
                                gs.bullet_moving = true;
                                gs.bullet_x = gs.cur_x;
                                gs.bullet_y = gs.cur_y;
                                sprites[2].to_region = [gs.bullet_x, gs.bullet_y, SPRITE_SIZE/4.0, SPRITE_SIZE/4.0];


                                if gs.bullet_count == 2{
                                    gs.bullet2_x = gs.cur_x;
                                    gs.bullet2_y = gs.cur_y;
                                    sprites[3].to_region = [gs.bullet2_x, gs.bullet2_y, SPRITE_SIZE/4.0, SPRITE_SIZE/4.0];
                                }  

                                if gs.bullet_count == 3{
                                    gs.bullet3_x = gs.cur_x;
                                    gs.bullet3_y = gs.cur_y;
                                    sprites[4].to_region = [gs.bullet3_x, gs.bullet3_y, SPRITE_SIZE/4.0, SPRITE_SIZE/4.0];
                                }             


                                }

                            // allows left and right movement simultaneously
                            // if input.is_key_down(winit::event::VirtualKeyCode::Space){
                            //     gs.bullet_moving = true;
                            //     gs.bullet_x = gs.cur_x;
                            //     gs.bullet_y = gs.cur_y;
                            //     sprites[2].to_region = [gs.bullet_x, gs.bullet_y, SPRITE_SIZE, SPRITE_SIZE];
                            // }
                        }

                        else if input.is_key_down(winit::event::VirtualKeyCode::Right){
                            //println!("Right");
                            gs.cur_x += 6.0;
                            sprites[1].to_region = [gs.cur_x, 0.0, SPRITE_SIZE, SPRITE_SIZE];
                            //println!("{}", gs.cur_x);

                            if input.is_key_down(winit::event::VirtualKeyCode::Space){
                                //println!("Space");
                                gs.bullet_count +=1;
      
                                // //new 
                                // let mut new_sprite = GPUSprite { to_region:
                                //     [gs.cur_x, 
                                //     gs.cur_y, 
                                //     // initially invisible
                                //     SPRITE_SIZE, 
                                //     SPRITE_SIZE],
                                // from_region: [
                                //     0.5, 
                                //     0.9,
                                //     0.25,
                                //     0.1]
                                // };

                                // let mut splice_num = gs.bullet_count + 3;
                                // gs.bullet_index = splice_num;
                                // sprites[splice_num] = new_sprite;
                                // //println!("{:#?}", new_sprite);
                                // gs.bullet_moving = true;
    
    
    
                                // // working VVVV
                                //println!("{}", gs.bullet_count.to_string());
      
                                // check if 0 bullets on screen
                                //println!("BRUHH");
                                gs.bullet_moving = true;
                                gs.bullet_x = gs.cur_x;
                                gs.bullet_y = gs.cur_y;
                                sprites[2].to_region = [gs.bullet_x, gs.bullet_y, SPRITE_SIZE, SPRITE_SIZE];
                                // increment bullet counter
                                gs.bullet_count+=1;


                                if gs.bullet_count == 2{
                                    gs.bullet2_x = gs.cur_x;
                                    gs.bullet2_y = gs.cur_y;
                                    sprites[3].to_region = [gs.bullet2_x, gs.bullet2_y, SPRITE_SIZE, SPRITE_SIZE];
                                }  

                                if gs.bullet_count == 3{
                                    gs.bullet3_x = gs.cur_x;
                                    gs.bullet3_y = gs.cur_y;
                                    sprites[4].to_region = [gs.bullet3_x, gs.bullet3_y, SPRITE_SIZE, SPRITE_SIZE];
                                }             

                                
                                }
 
                        } else if input.is_key_down(winit::event::VirtualKeyCode::Space){
                                //println!("Space");
                                gs.bullet_count +=1;
      
                                // //new 
                                // let mut new_sprite = GPUSprite { to_region:
                                //     [gs.cur_x, 
                                //     gs.cur_y, 
                                //     // initially invisible
                                //     SPRITE_SIZE, 
                                //     SPRITE_SIZE],
                                // from_region: [
                                //     0.5, 
                                //     0.9,
                                //     0.25,
                                //     0.1]
                                // };

                                // let mut splice_num = gs.bullet_count + 3;
                                // gs.bullet_index = splice_num;
                                // sprites[splice_num] = new_sprite;
                                // //println!("{:#?}", new_sprite);
                                // gs.bullet_moving = true;
    
    
    
                                // // working VVVV
                                //println!("{}", gs.bullet_count.to_string());
      
                                // check if 0 bullets on screen
                                //println!("BRUHH");
                                gs.bullet_moving = true;
                                gs.bullet_x = gs.cur_x;
                                gs.bullet_y = gs.cur_y;
                                sprites[2].to_region = [gs.bullet_x, gs.bullet_y, SPRITE_SIZE, SPRITE_SIZE];
                                // increment bullet counter
                                gs.bullet_count+=1;


                                if gs.bullet_count == 2{
                                    gs.bullet2_x = gs.cur_x;
                                    gs.bullet2_y = gs.cur_y;
                                    sprites[3].to_region = [gs.bullet2_x, gs.bullet2_y, SPRITE_SIZE, SPRITE_SIZE];
                                }  

                                if gs.bullet_count == 3{
                                    gs.bullet3_x = gs.cur_x;
                                    gs.bullet3_y = gs.cur_y;
                                    sprites[4].to_region = [gs.bullet3_x, gs.bullet3_y, SPRITE_SIZE, SPRITE_SIZE];
                                }             

                                
                                }
                        
       
                        // any bullets shot GOOD AND WORKING VVVVVV
                        if gs.bullet_moving{
                            // for sprite in &mut sprites {
                            //     // Perform your operation here - add y vel
                            //     let mut graivty = sprites.to_region[1] + gs.bullet_speed;
                            //     //println!(
                            //     "{}", graivty
                            //     );
                            //      // render sprite
                            //     sprites[gs.bullet_index].to_region = [sprite.to_region[0], graivty, SPRITE_SIZE, SPRITE_SIZE];
                            // }

                            // working VVVVVVVVVVVVVVVVVVVVVVVVV

                            // USING WAITING TO CHECK IF BULLET SHOT
                            if gs.bullet_y < WINDOW_HEIGHT {
                                
                                //println!("RUH ROH");
                                //cur_y = cur_y + bullet_speed;
                                gs.bullet_y += gs.bullet_speed;
                                //sprites[gs.bullet_index].to_region = [sprites[gs.bullet_index].to_region[0], sprites[gs.bullet_index].to_region[1] + gs.bullet_speed, SPRITE_SIZE, SPRITE_SIZE];
                            
                                sprites[2].to_region = [gs.bullet_x, gs.bullet_y, SPRITE_SIZE, SPRITE_SIZE];
                                let targetx: f32 = sprites[0].to_region[0]; 
                                
                                if (((gs.bullet_x >= targetx-SPRITE_SIZE)&(gs.bullet_x <= targetx + SPRITE_SIZE))  & (gs.bullet_y >= WINDOW_HEIGHT-SPRITE_SIZE-50.0))  { 
                                    hit = true;
                                        
                                }
                        
                            }
                            ////////  VVVV
                            if gs.bullet2_y < WINDOW_HEIGHT {
                                
                                //println!("RUH ROH");
                                //cur_y = cur_y + bullet_speed;
                                gs.bullet2_y += gs.bullet_speed;
                                //sprites[gs.bullet_index].to_region = [sprites[gs.bullet_index].to_region[0], sprites[gs.bullet_index].to_region[1] + gs.bullet_speed, SPRITE_SIZE, SPRITE_SIZE];
                            
                                sprites[3].to_region = [gs.bullet2_x, gs.bullet2_y, SPRITE_SIZE, SPRITE_SIZE];
                                let targetx: f32 = sprites[0].to_region[0]; 
                                
                                if (((gs.bullet2_x >= targetx-SPRITE_SIZE)&(gs.bullet2_x <= targetx + SPRITE_SIZE))  & (gs.bullet2_y >= WINDOW_HEIGHT-SPRITE_SIZE-50.0))  { 
                                    hit2 = true;
                                        
                                }
                            }   

                            if gs.bullet3_y < WINDOW_HEIGHT {
                                
                                //println!("RUH ROH");
                                //cur_y = cur_y + bullet_speed;
                                gs.bullet3_y += gs.bullet_speed;
                                //sprites[gs.bullet_index].to_region = [sprites[gs.bullet_index].to_region[0], sprites[gs.bullet_index].to_region[1] + gs.bullet_speed, SPRITE_SIZE, SPRITE_SIZE];
                            
                                sprites[4].to_region = [gs.bullet3_x, gs.bullet3_y, SPRITE_SIZE, SPRITE_SIZE];
                                let targetx: f32 = sprites[0].to_region[0]; 
                                
                                if (((gs.bullet3_x >= targetx-SPRITE_SIZE)&(gs.bullet3_x <= targetx + SPRITE_SIZE))  & (gs.bullet3_y >= WINDOW_HEIGHT-SPRITE_SIZE-50.0))  { 
                                    hit3 = true;
                                        
                                }
                            
                            }

                            // }
                            
                            if (hit){
                                gs.score += 1;
                                gs.bullet_count -= 1;
                                // this will reset the sprite after hitting the target
                                gs.bullet_y = WINDOW_HEIGHT;
                                gs.bullet_moving = false;


                                //println!("Hit{}" , hits);
                                
                                let  x: f32 = rng.gen_range((0.0 ) .. 10.0 );
                                        let sign: f32 = rng.gen_range(0.0.. 3.0);
                                        gs.target_x = sprites[0].to_region[0];
                                        if ((sign <1.0) & (gs.target_x < WINDOW_WIDTH - 10.0)){
                                            gs.target_x += x;
                                            
                                        }  else if((sign > 1.0 )& (gs.target_x > 10.0)) {
                                           gs.target_x -= x;
                                        } 
                                        sprites[0].to_region = [gs.target_x, WINDOW_HEIGHT-SPRITE_SIZE, SPRITE_SIZE, SPRITE_SIZE]; 
                                    
                                    }     

                            if (hit2){
                                gs.score += 1;
                                gs.bullet_count -= 1;
                                // this will reset the sprite after hitting the target
                                gs.bullet2_y = WINDOW_HEIGHT;
                                gs.bullet_moving = false;


                                //println!("Hit{}" , hits);
                                
                                let  x: f32 = rng.gen_range((0.0 ) .. 10.0 );
                                        let sign: f32 = rng.gen_range(0.0.. 3.0);
                                        gs.target_x = sprites[0].to_region[0];
                                        if ((sign <1.0) & (gs.target_x < WINDOW_WIDTH - 10.0)){
                                            gs.target_x += x;
                                            
                                        }  else if((sign > 1.0 )& (gs.target_x > 10.0)) {
                                           gs.target_x -= x;
                                        } 
                                        sprites[0].to_region = [gs.target_x, WINDOW_HEIGHT-SPRITE_SIZE, SPRITE_SIZE, SPRITE_SIZE]; 
                                    
                                    } 
                            if (hit3){
                                gs.score += 1;
                                gs.bullet_count -= 1;
                                // this will reset the sprite after hitting the target
                                gs.bullet3_y = WINDOW_HEIGHT;
                                gs.bullet_moving = false;


                                //println!("Hit{}" , hits);
                                
                                
                                    } 
   
                                    
                        }


                        }
                    



                        

////////////////////////////////////////////////////////////////////////////////////////////////////
            }
            else if gs.screen == 1 {
                buffer.set_text(&mut font_system, "", Attrs::new().family(Family::Serif), Shaping::Advanced);
                // Do we need to show new sprites?
                if gs.waiting == false && gs.falling == false{
                    // game restart
                    if gs.num_stacked > 11{
                        if !input.is_key_down(winit::event::VirtualKeyCode::Space){
                            let new_level = gs.level + 1;
                            let new_speed = gs.speed + 1;
                            // Reset gs variables manually to reduce cross game variable errors
                            gs = game_state::init_game_state();
                            gs.screen = 1;
                            gs.level = new_level;
                            if game_mode == 1 {
                                gs.drop_sprite_blocks = 5;
                                gs.speed = new_speed;
                            }else if game_mode == 2{
                                gs.drop_sprite_blocks = 4;
                                gs.speed = new_speed;
                            }else{ // game_mode == 3
                                gs.drop_sprite_blocks = 3;
                                gs.speed = new_speed;
                            }
                            sprites = (0..gs.drop_sprite_blocks*12).map(|_| GPUSprite{
                                to_region: 
                                    [WINDOW_WIDTH,
                                    WINDOW_HEIGHT,
                                    0.0, // generate width and height to be 0 so that you can adjust later, but are now invisible
                                    0.0], 
                                from_region:[
                                    0.25, 
                                    0.0, 
                                    0.25,
                                    0.1],
                            }).collect();

                            // write next level text on the screen (display level for a second?)
                        }
                    }else if gs.drop_sprite_blocks == 0{
                        gs = game_state::init_game_state();
                        gs.screen = 3; // go to game over screen
                    }
                    let mut i:usize = gs.sprites_used;
                    // XPOS OF LEFTMOST SPRITE
                    let x_pos = rng.gen_range(0..WINDOW_WIDTH as usize-(SPRITE_SIZE as usize*gs.drop_sprite_blocks));
                    // chooe a random color on the sprite sheet for this row that will drop
                    let color_loc: (f32, f32) = (
                        0.25 + rng.gen_range(0..2) as f32*0.25,
                        0.0 + rng.gen_range(0..10) as f32*0.1);
                    while i < gs.drop_sprite_blocks + gs.sprites_used{
                        sprites[i].to_region = [
                            x_pos as f32+(((i-gs.sprites_used)*64) as f32), 
                            WINDOW_HEIGHT - SPRITE_SIZE, 
                            SPRITE_SIZE, 
                            SPRITE_SIZE];
                        sprites[i].from_region = [
                            color_loc.0, 
                            color_loc.1,
                            0.25,
                            0.1];
                        i += 1;
                    }
                    gs.sprites_used += gs.drop_sprite_blocks;
                    gs.waiting = true;
                // Do we need to animate falling sprite
                }else if gs.falling == true{
                    let mut still_falling = false;
                    for sprite in &mut sprites {
                        let cur_y = sprite.to_region[1];
                        // if it has not yet fallen below the level it will fall to, keep falling
                        if cur_y >= 0.0 + gs.num_stacked as f32*SPRITE_SIZE && cur_y < WINDOW_HEIGHT{
                            still_falling = true;
                            sprite.to_region = [sprite.to_region[0], cur_y - gs.speed as f32/2.0, SPRITE_SIZE, SPRITE_SIZE];
                        }
                    }
                    if !still_falling{
                        gs.falling = false;
                        gs.num_stacked += 1;
                    }
                    // We are waiting for space to be clicked, and then acting on it
                }else{
                    if input.is_key_down(winit::event::VirtualKeyCode::Space){
                        buffer.set_text(&mut font_system, "", Attrs::new().family(Family::Serif), Shaping::Advanced);
                        let mut left_edge = WINDOW_WIDTH;
                        let mut right_edge = 0.0;
                        for sprite in &mut sprites {
                            if sprite.to_region[1] == WINDOW_HEIGHT-SPRITE_SIZE{
                                if sprite.to_region[0] < left_edge {
                                    left_edge = sprite.to_region[0];
                                }
                                if sprite.to_region[0] > right_edge {
                                    right_edge = sprite.to_region[0];
                                }
                                ////println!("left: {} right: {}", left_edge, right_edge);
                                if sprite.to_region[0] < (gs.left_border - SPRITE_SIZE/2.0){
                                    sprite.to_region = [
                                        100.0, 
                                        WINDOW_HEIGHT, 
                                        0.0, 
                                        0.0];
                                    gs.drop_sprite_blocks -= 1;
                                }
                                if sprite.to_region[0] > (gs.right_border + SPRITE_SIZE/2.0){
                                    sprite.to_region = [
                                        100.0, 
                                        WINDOW_HEIGHT, 
                                        0.0, 
                                        0.0];
                                    gs.drop_sprite_blocks -= 1;
                                }
                            }
                        }
                        // now update the edges of the game state for the next frame
                        if left_edge > gs.left_border {
                            gs.left_border = left_edge;
                        }
                        if right_edge < gs.right_border {
                            gs.right_border = right_edge;
                        }


                        gs.waiting = false;
                        gs.falling = true;
                    }else{
                        //ANIMATE BACK AND FORTH
                        // direction = true when going left
                        // consider adding active field to sprites
                        let mut delta = gs.speed as f32;
                        if gs.direction == true{
                            delta = gs.speed as f32 * (-1.0);
                        }
                        if gs.num_stacked == 0{
                            let text_1 = "Level: ";
                            let text = text_1.to_owned() + &gs.level.to_string();
                            buffer.set_text(&mut font_system, &text, Attrs::new().family(Family::Serif), Shaping::Advanced);
                        }

                        for sprite in &mut sprites {
                            if sprite.to_region[1] == WINDOW_HEIGHT - SPRITE_SIZE{
                                let cur_x = sprite.to_region[0];
                                if cur_x >= 960.0 - delta{
                                    gs.direction = true;
                                }else if cur_x < 0.0 + delta{
                                    gs.direction = false
                                }
                                sprite.to_region = [cur_x + delta, WINDOW_HEIGHT - SPRITE_SIZE, SPRITE_SIZE, SPRITE_SIZE];
                            }                    
                        }
                    }

                    // SCREEN == 5 -> SPACE GAME LOOP
                }
            } else if gs.screen == 2 {   
                // Block falling game setup screen
                gs = game_state::init_game_state();
                gs.screen = 2;
                // reset sprites
                sprites = (0..gs.drop_sprite_blocks*12).map(|_| GPUSprite{
                    to_region: [WINDOW_WIDTH,WINDOW_HEIGHT,0.0, 0.0], 
                    from_region:[0.25, 0.0, 0.25,0.1],}).collect();
                // Text for setup 
                buffer.set_text(&mut font_system, "Press a key to choose your difficulty level:\n1:EASY\n2:INTERMEDIATE\n3:ADVANCED", Attrs::new().family(Family::Serif), Shaping::Advanced);
                // input logic
                if input.is_key_down(winit::event::VirtualKeyCode::Key1){
                    game_mode = 1;
                    gs.screen = 1;
                }else if input.is_key_down(winit::event::VirtualKeyCode::Key2){
                    game_mode = 2;
                    gs.screen = 1;
                }else if input.is_key_down(winit::event::VirtualKeyCode::Key3){
                    game_mode = 3;
                    gs.screen = 1;
                }
                if game_mode == 1 {
                    gs.drop_sprite_blocks = 5;
                    gs.speed = 4;
                }else if game_mode == 2{
                    gs.drop_sprite_blocks = 4;
                    gs.speed = 6;
                }else{ // game_mode == 3
                    gs.drop_sprite_blocks = 3;
                    gs.speed = 10;
                }

            } else if gs.screen == 3{
                // reset sprites
                sprites = (0..gs.drop_sprite_blocks*12).map(|_| GPUSprite{
                    to_region: [WINDOW_WIDTH,WINDOW_HEIGHT,0.0, 0.0], 
                    from_region:[0.25, 0.0, 0.25,0.1],}).collect();
                // Block falling game over screen
                buffer.set_text(&mut font_system, "GAME OVER!!!\nPress c to continue playing this game\nPress esc for title screen", Attrs::new().family(Family::Serif), Shaping::Advanced);
                if input.is_key_down(winit::event::VirtualKeyCode::C){
                    gs = game_state::init_game_state();
                    gs.screen = 2;
                }
            }else if gs.screen == 4{

            //}else if gs.screen == 5{
                // buffer.set_text(&mut font_system, "", Attrs::new().family(Family::Serif), Shaping::Advanced);
                // // space game
                // //println!("GAME 2!!!!");
                // gs.screen = 5;

                // if gs.start{
                // // target sprite
                // sprites[0].to_region = [
                //     500.0, 
                //     WINDOW_HEIGHT - SPRITE_SIZE, 
                //     SPRITE_SIZE, 
                //     SPRITE_SIZE];
                // sprites[0].from_region = [
                //     0.75, 
                //     0.0,
                //     0.25,
                //     0.1];

                // // ship sprite VVV
                // sprites[1].to_region = [
                //     gs.cur_x, 
                //     gs.cur_y, 
                //     SPRITE_SIZE, 
                //     SPRITE_SIZE];
                // sprites[1].from_region = [
                //     0.75, 
                //     0.9,
                //     0.25,
                //     0.1];

                // // Bullet SPrite - initially invisible
                // sprites[2].to_region = [
                //     sprites[1].to_region[0], 
                //     sprites[1].to_region[1], 
                //     // initially invisible
                //     0.0, 
                //     0.0];
                // sprites[2].from_region = [
                //     0.5, 
                //     0.9,
                //     0.25,
                //     0.1];

                // // checks left and right movement
                // if input.is_key_down(winit::event::VirtualKeyCode::Left){
                //     //println!("Left");
                //     gs.cur_x -= 6.0;
                //     sprites[1].to_region = [gs.cur_x, 0.0, SPRITE_SIZE, SPRITE_SIZE];
                //     //println!("{}", gs.cur_x)
                // }

                // else if input.is_key_down(winit::event::VirtualKeyCode::Right){
                //     //println!("Right");
                //     gs.cur_x += 6.0;
                //     sprites[1].to_region = [gs.cur_x, 0.0, SPRITE_SIZE, SPRITE_SIZE];
                //     //println!("{}", gs.cur_x)
                // }

                // else if input.is_key_down(winit::event::VirtualKeyCode::Space){
                //     //println!("SHOOTING");
                //     // USING WAITING TO CHECK IF BULLET SHOT
                //     gs.bullet_moving = true;
                //     gs.bullet_x = gs.cur_x;
                //     gs.bullet_y = gs.cur_y;
                //     sprites[2].to_region = [gs.bullet_x, gs.bullet_y, SPRITE_SIZE, SPRITE_SIZE];


                // }

                // // if gs.bullet_moving{

                // //     //println!("SHOOTING");
                // //     // USING WAITING TO CHECK IF BULLET SHOT
                // //     if gs.bullet_y < WINDOW_HEIGHT {
                // //     //cur_y = cur_y + bullet_speed;
                // //     gs.bullet_y += gs.bullet_speed;
                // //     //println!("{}", gs.cur_y);
                // //     sprites[2].to_region = [gs.bullet_x, gs.bullet_y, SPRITE_SIZE, SPRITE_SIZE];

                // //     }
                // //     else{
                // //         gs.bullet_moving = false;
                // //     }
                // // }


                // }
            
            }else if gs.screen == 6{

            }

            // Text rendering
            text_renderer.prepare(
                &device,
                &queue,
                &mut font_system,
                &mut atlas,
                Resolution {
                    width: config.width,
                    height: config.height,
                },
                [TextArea {
                    buffer: &buffer,
                    left: 150.0,
                    top: 200.0,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: 50000,
                        bottom: 100000,
                    },
                    default_color: Color::rgb(255, 255, 255),
                }],
                &mut cache,
            ).unwrap();

            // Remember this from before?
            //input.next_frame();
            queue.write_buffer(&buffer_camera, 0, bytemuck::bytes_of(&camera));
            queue.write_buffer(&buffer_sprite, 0, bytemuck::cast_slice(&sprites));

            let frame = surface
                .get_current_texture()
                .expect("Failed to acquire next swap chain texture");
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                    // timestamp_writes: None,
                    // occlusion_query_set: None,
                });
                text_renderer.render(&atlas, &mut rpass).unwrap();
                rpass.set_pipeline(&render_pipeline);
                rpass.set_bind_group(0, &sprite_bind_group, &[]);
                rpass.set_bind_group(1, &texture_bind_group, &[]);

                // draw two triangles per sprite, and sprites-many sprites.
                // this uses instanced drawing, but it would also be okay
                // to draw 6 * sprites.len() vertices and use modular arithmetic
                // to figure out which sprite we're drawing, instead of the instance index.
                rpass.draw(0..6, 0..(sprites.len() as u32));
            } 
            

            queue.submit(Some(encoder.finish()));
            frame.present();
            
            window.request_redraw();
            input.next_frame();
            atlas.trim();
        },
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        _ => {}
    }
    window.request_redraw();

    });

    
}





// Main is just going to configure an event loop, open a window, set up logging, and kick off our `run` function.
fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        // On native, we just want to wait for `run` to finish.
        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        // On web things are a little more complicated.
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        // Now we use the browser's runtime to spawn our async run function.
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }

    
}


// AsRef means we can take as parameters anything that cheaply converts into a Path,
// for example an &str.
fn load_texture(
    path: impl AsRef<std::path::Path>,
    label: Option<&str>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(wgpu::Texture, image::RgbaImage), image::ImageError> {
    // This ? operator will return the error if there is one, unwrapping the result otherwise.
    let img = image::open(path.as_ref())?.to_rgba8();
    let (width, height) = img.dimensions();
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &img,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );
    Ok((texture, img))
}
