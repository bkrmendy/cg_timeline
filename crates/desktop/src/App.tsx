import React from "react";
import "./App.css";
import styled from "styled-components";

const Container = styled.div`
  margin: 0;
  display: flex;
  flex-direction: row;
  text-align: center;
  height: 100vh;
`;

const Main = styled.div`
  display: flex;
  flex-direction: column;
  text-align: center;
  align-items: center;
  justify-content: center;
  gap: 10px;

  width: 100%;
  height: 100%;
  padding: 20px;
`;

function App() {
  return (
    <Container>
      <Main>Server is running!</Main>
    </Container>
  );
}

export default App;
