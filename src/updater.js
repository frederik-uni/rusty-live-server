window.onload = () => {
  const socket = new WebSocket(`ws://${window.location.host}/ws`);

  socket.onmessage = (event) => {
    console.log(event);

    if (event.data === "reload") {
      window.location.reload();
    }
  };

  socket.onopen = () => {
    console.log("Reload WebSocket connection established");
  };

  socket.onerror = (error) => {
    console.error("Reload WebSocket error:", error);
  };
};
