program world_saver;

// if true, then all houses are saved as custom
// if false, then all houses are saved as standard
// yet it is not possible to determine standard / custom houses
const custom_house = true;

// if true, in addition to houses, all other items will be saved, except for mobiles
const save_items = true;

// time in seconds between sending data, during this time the data will be accumulated in the cache
const send_interval = 2.5;

// if false ignore saved items, only between data sending, if true once sent items are always ignored 
const forever_ignore = false;


function get_item_json(serial: cardinal; flag: cardinal): string;
var 
  x, y, z, graphic, world: integer;
    
begin
  world := WorldNum;
  x := GetX(serial);
  y := GetY(serial);
  z := GetZ(serial);
  graphic := GetType(serial) or flag;
  
  if (x = 0) or (y = 0) or (graphic = 0) then 
  begin
    AddToSystemJournal('skip incomplete item: ', serial);
    exit('');
  end;
  
  result := Format('{"world": %d, "serial": %d, "x": %d, "y": %d, "z": %d, "graphic": %d}', [world, serial, x, y, z, graphic]);
  Ignore(serial);
  //AddToSystemJournal(result); 
end;


function get_house_part_json(part: TMultiPart): string;
begin
  result := Format('{"x": %d, "y": %d, "z": %d, "graphic": %d, "flags": %d}', [part.x, part.y, part.z, part.graphic, part.flag]);
end;


function get_house_json(serial: cardinal): string;
var parts: TMultiParts;
    i: integer;
    house_item: string;
    house_part, house_parts: string;
    
begin
  house_item := get_item_json(serial, $20000);
  parts := GetMultiAllParts(serial);
  
  if house_item == '' then exit('');
  
  house_parts := '';
  for i:=0 to length(parts)-1 do
  begin
    house_part := get_house_part_json(parts[i]);
    if house_parts <> '' then
    begin
      house_parts := house_parts + ', '+house_part
    end
    else
    begin
      house_parts := house_part;
    end;
  end;
  
  result := Format('{"item": %s, "parts": [%s]}', [house_item, house_parts]);
  Ignore(serial);
end;


procedure to_cache(var cache: string; json: string);
begin
  if json == '' then exit;
  if cache == '' then
     cache := json
  else
    cache := cache +', ' + json;    
end;



var items: TCardinalDynArray;
    item_cache: string;
    multi_cache: string;
    i : integer;
    json: string;
    last_send: Double;
    
begin
  FindDistance := 36;
  FindVertical := 127;
  
  last_send := now();

  while true do
  begin
    FindType(0xFFFF, Ground);
    items := GetFoundItems();
   
    for i := 0 to length(items)-1 do
    begin
      if IsNPC(items[i]) then continue;
    
      if IsHouse(items[i]) then begin
        if custom_house then
        begin
          json := get_house_json(items[i]);
          to_cache(multi_cache, json);
        end
        else
        begin
          json := get_item_json(items[i], $10000);
          to_cache(item_cache, json);
        end;
      end
      else
      begin
        if save_items then
        begin
          json := get_item_json(items[i], 0);
          to_cache(item_cache, json);
        end;
      end;
    end;
    
    
    if now() > (last_send+send_interval/(60*60*24)) then
    begin
      if multi_cache <> '' then
      begin
        json := Format('{"MultiItemsAdd": {"multi_items": [%s]}}', [multi_cache]);
        HTTP_Post('http://127.0.0.1:3000/api/', json);
        
        multi_cache := '';
      end;
    

      if item_cache <> '' then
      begin
        json := Format('{"ItemsAdd": {"items": [%s]}}', [item_cache]);
        HTTP_Post('http://127.0.0.1:3000/api/', json);
        
        item_cache := '';
      end;
      
      if not forever_ignore then
        IgnoreReset();
    
      last_send := now();
    end;    
    
    Wait(50);
  end;
end.
