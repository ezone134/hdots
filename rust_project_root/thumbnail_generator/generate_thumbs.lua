local ffi = require("ffi")

-- 1. Setup paths
local home = os.getenv("HOME") or "/home/tw"
local lib_path = home .. "/hypr_bin/lib/libstb.so"
local source_dir = home .. "/Wallpapers"
-- local cache_dir = (os.getenv("rcache") or (home .. "/.cache/hypr")) .. "/wall_thumbs"
local cache_dir = home .. "/.cache/thumbnails/hdots/wall_thumbs"

-- 2. FFI Definitions for stb_image_resize2.h
-- Note: stbir_resize_uint8_linear is the standard linear-filter variant in resize2
ffi.cdef [[
    unsigned char *stbi_load(const char *filename, int *x, int *y, int *channels, int desired_channels);
    void stbi_image_free(void *data);
    int stbi_write_png(const char *filename, int w, int h, int comp, const void *data, int stride_in_bytes);
    int stbir_resize_uint8_linear(const unsigned char *input_pixels, int input_w, int input_h, int input_stride_in_bytes,
                                  unsigned char *output_pixels, int output_w, int output_h, int output_stride_in_bytes,
                                  int num_channels);
]]

local stb = ffi.load(lib_path)
os.execute("mkdir -p " .. cache_dir)

-- 3. Thumbnail Generation Function
local function create_thumb(input_path, output_path, thumb_w, thumb_h)
    local w, h, c = ffi.new("int[1]"), ffi.new("int[1]"), ffi.new("int[1]")
    local data = stb.stbi_load(input_path, w, h, c, 3)

    if data == nil then return false end

    local out_data = ffi.new("unsigned char[?]", thumb_w * thumb_h * 3)

    -- Updated to use resize2 linear function
    stb.stbir_resize_uint8_linear(data, w[0], h[0], 0, out_data, thumb_w, thumb_h, 0, 3)
    stb.stbi_write_png(output_path, thumb_w, thumb_h, 3, out_data, 0)

    stb.stbi_image_free(data)
    return true
end

-- 4. Main Execution (Dependency-free scanning)
print("Scanning: " .. source_dir)

-- Using ls -1 to list files; handles symlinks correctly by default
local handle = io.popen('ls -1 "' .. source_dir .. '"')
if handle then
    for file in handle:lines() do
        local in_path = source_dir .. "/" .. file
        local ext = file:lower():match("%.(%a+)$")

        -- Only process supported image formats
        if ext == "jpg" or ext == "jpeg" or ext == "png" or ext == "gif" then
            local out_path = cache_dir .. "/" .. file

            -- Check if file exists using system 'test' command (no lfs needed)
            local check = os.execute('test -f "' .. out_path .. '"')
            if check ~= 0 then
                print("Processing: " .. file)
                if create_thumb(in_path, out_path, 200, 200) then
                    print("Generated: " .. out_path)
                else
                    print("Failed to process: " .. file)
                end
            end
        end
    end
    handle:close()
end

print("Done.")
