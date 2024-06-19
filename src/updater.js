window.onload = () => {
  const socket = new WebSocket(`ws://${window.location.host}/ws`);

  socket.onmessage = (event) => {
    if (event.data === "reload") {
      window.location.reload();
    }
    if (event.data.startsWith("update-css://")) {
      const filePath = event.data.substring(13);
      console.log(filePath);

      const links = document.getElementsByTagName("link");
      for (const link of links) {
        if (link.rel !== "stylesheet") continue;
        const clonedLink = link.cloneNode(true);
        if (link.href.startsWith("http://127.0.0.1:8081" + filePath)) {
          if (link.href.includes("?")) {
            const indexOf = link.href.indexOf("?");
            if (link.href.slice(indexOf).contains("counter=")) {
              //todo: increment
            } else {
              clonedLink.href += "&counter=1";
            }
          } else {
            clonedLink.href += "?counter=1";
          }
        }
        link.replaceWith(clonedLink);
      }
    }
  };

  socket.onopen = () => {
    console.log("Reload WebSocket connection established");
  };

  socket.onerror = (error) => {
    console.error("Reload WebSocket error:", error);
  };
};
