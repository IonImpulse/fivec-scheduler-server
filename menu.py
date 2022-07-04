url = "https://legacy.cafebonappetit.com/api/2/cafes?cafe="

for i in range(3001, 4000):
    url = url + str(i) + ","

print(url)