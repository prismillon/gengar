FROM python:3.11-slim-bullseye
RUN apt update && apt install git -y
WORKDIR /app
COPY requirements.txt ./
RUN pip install --upgrade pip
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
CMD [ "python3", "/app/gengar.py" ]