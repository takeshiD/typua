local total = 0
for i = 1, 3, 1 do
	print(i)
end
for k, v in pairs(t) do
	print(k, v)
end
if total > 0 then
	total = total - 1
elseif total == 0 then
	total = 10
else
	total = -total
end
while total < 5 do
	total = total + 1
end
repeat
	total = total - 2
until total == 0
do
	local inner = 1
end
foo:bar(1, 2)
return total
