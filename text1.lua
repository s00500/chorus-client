function script_description()
	return "This script lets you select a text source to force it to reread files every frame instead of once per second."
end

obs = obslua
source_name = nil

local open = io.open

local function read_file(path)
    local file = open(path, "rb") -- r read mode and b binary mode
    if not file then return nil end
    local content = file:read "*a" -- *a or *all reads the whole file
    file:close()
    return content
end





function script_update(settings)
  source_name = obs.obs_data_get_string(settings, "source")
  filepath = obs.obs_data_get_string(settings, "filepath")

  local fileContent = read_file("/Users/LB/Desktop/z_Projects/chorus-client/racestate.txt");
  set_text(fileContent)
end

function script_properties()
	local props = obs.obs_properties_create()
  local p = obs.obs_properties_add_list(props, "source", "Text Source", obs.OBS_COMBO_TYPE_EDITABLE, obs.OBS_COMBO_FORMAT_STRING)
  local p2 = obs.obs_properties_add_path(props, "filepath", "File", obs.OBS_PATH_FILE, nil, nil)

	local sources = obs.obs_enum_sources()
	if sources ~= nil then
		for _, source in ipairs(sources) do
			source_id = obs.obs_source_get_id(source)
			if source_id == "text_gdiplus" or source_id == "text_ft2_source" or source_id == "text_pango_source" then
				local name = obs.obs_source_get_name(source)
				obs.obs_property_list_add_string(p, name, name)
			end
		end
  end

	obs.source_list_release(sources)

	return props
end

function set_text(text)

    local source = obs.obs_get_source_by_name(source_name)

		if source ~= nil then
			local settings = obs.obs_data_create()
			obs.obs_data_set_string(settings, "text", text)
			obs.obs_source_update(source, settings)
			obs.obs_data_release(settings)
			obs.obs_source_release(source)
		end
  end

function script_tick(seconds)
	if source_name == nil then return end
  local fileContent = read_file(filepath);
  set_text(fileContent)
end
