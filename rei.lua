local mp = require('mp')

function on_pause_change(name, value)
  if value == true then
      stop_start(value)
end 
end 


function stop_start(value)
  if value == true then
    
elseif value == false then 
end

end

mp.observe_property('pause', 'bool', on_pause_change)
