const slider = document.getElementById("slider");
const valueLabel = document.getElementById("value");

slider.addEventListener("input", () => {
  const value = slider.value;
  valueLabel.textContent = value;

  fetch("/set", {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded"
    },
    body: "value=" + value
  })
  .catch(err => console.error("Error sending data:", err));
});
