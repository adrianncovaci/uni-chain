import React from 'react'
import {
  Button,
  Card,
  Grid,
  Modal,
  Form,
  Label,
} from 'semantic-ui-react'

import CourseAvatar from './CourseAvatar'
import { useSubstrateState } from './substrate-lib'
import { TxButton } from './substrate-lib/components'

// --- Transfer Modal ---

const TransferModal = props => {
  const { course, setStatus } = props
  const [open, setOpen] = React.useState(false)
  const [formValue, setFormValue] = React.useState({})

  const formChange = key => (ev, el) => {
    setFormValue({ ...formValue, [key]: el.value })
  }

  const confirmAndClose = unsub => {
    setOpen(false)
    if (unsub && typeof unsub === 'function') unsub()
  }

  return (
    <Modal
      onClose={() => setOpen(false)}
      onOpen={() => setOpen(true)}
      open={open}
      trigger={
        <Button basic color="blue">
          Transfer
        </Button>
      }
    >
      <Modal.Header>Course Transfer</Modal.Header>
      <Modal.Content>
        <Form>
          <Form.Input fluid label="Course ID" readOnly value={course.dna} />
          <Form.Input
            fluid
            label="Receiver"
            placeholder="Receiver Address"
            onChange={formChange('target')}
          />
        </Form>
      </Modal.Content>
      <Modal.Actions>
        <Button basic color="grey" onClick={() => setOpen(false)}>
          Cancel
        </Button>
        <TxButton
          label="Transfer"
          type="SIGNED-TX"
          setStatus={setStatus}
          onClick={confirmAndClose}
          attrs={{
            palletRpc: 'courseGrading',
            callable: 'transfer',
            inputParams: [formValue.target, course.dna],
            paramFields: [true, true],
          }}
        />
      </Modal.Actions>
    </Modal>
  )
}

// --- Set Price ---

const SetPrice = props => {
  const { course, setStatus } = props
  const [open, setOpen] = React.useState(false)
  const [formValue, setFormValue] = React.useState({})

  const formChange = key => (ev, el) => {
    setFormValue({ ...formValue, [key]: el.value })
  }

  const confirmAndClose = unsub => {
    setOpen(false)
    if (unsub && typeof unsub === 'function') unsub()
  }

  return (
    <Modal
      onClose={() => setOpen(false)}
      onOpen={() => setOpen(true)}
      open={open}
      trigger={
        <Button basic color="blue">
          Set Price
        </Button>
      }
    >
      <Modal.Header>Set Course Price</Modal.Header>
      <Modal.Content>
        <Form>
          <Form.Input fluid label="Course ID" readOnly value={course.dna} />
          <Form.Input
            fluid
            label="Price"
            placeholder="Enter Price"
            onChange={formChange('target')}
          />
        </Form>
      </Modal.Content>
      <Modal.Actions>
        <Button basic color="grey" onClick={() => setOpen(false)}>
          Cancel
        </Button>
        <TxButton
          label="Set Price"
          type="SIGNED-TX"
          setStatus={setStatus}
          onClick={confirmAndClose}
          attrs={{
            palletRpc: 'courseGrading',
            callable: 'setPrice',
            inputParams: [course.dna, formValue.target],
            paramFields: [true, true],
          }}
        />
      </Modal.Actions>
    </Modal>
  )
}

// --- Buy Course ---

const BuyCourse = props => {
  const { course, setStatus } = props
  const [open, setOpen] = React.useState(false)

  const confirmAndClose = unsub => {
    setOpen(false)
    if (unsub && typeof unsub === 'function') unsub()
  }

  if (!course.price) {
    return <></>
  }

  return (
    <Modal
      onClose={() => setOpen(false)}
      onOpen={() => setOpen(true)}
      open={open}
      trigger={
        <Button basic color="green">
          Buy Course
        </Button>
      }
    >
      <Modal.Header>Buy Course</Modal.Header>
      <Modal.Content>
        <Form>
          <Form.Input fluid label="Course ID" readOnly value={course.dna} />
          <Form.Input fluid label="Price" readOnly value={course.price} />
        </Form>
      </Modal.Content>
      <Modal.Actions>
        <Button basic color="grey" onClick={() => setOpen(false)}>
          Cancel
        </Button>
        <TxButton
          label="Buy Course"
          type="SIGNED-TX"
          setStatus={setStatus}
          onClick={confirmAndClose}
          attrs={{
            palletRpc: 'courseGrading',
            callable: 'buyCourse',
            inputParams: [course.dna, course.price],
            paramFields: [true, true],
          }}
        />
      </Modal.Actions>
    </Modal>
  )
}

// --- About Course Card ---

const CourseCard = props => {
  const { course, setStatus } = props
  const { dna = null, owner = null, course_year = null, price = null } = course
  const displayDna = dna && dna.toJSON()
  const { currentAccount } = useSubstrateState()
  const isSelf = currentAccount.address === course.owner

  return (
    <Card>
      {isSelf && (
        <Label as="a" floating color="teal">
          Mine
        </Label>
      )}
      <CourseAvatar dna={dna.toU8a()} />
      <Card.Content>
        <Card.Meta style={{ fontSize: '.9em', overflowWrap: 'break-word' }}>
          DNA: {displayDna}
        </Card.Meta>
        <Card.Description>
          <p style={{ overflowWrap: 'break-word' }}>Course Year: {course_year}</p>
          <p style={{ overflowWrap: 'break-word' }}>Owner: {owner}</p>
          <p style={{ overflowWrap: 'break-word' }}>
            Price: {price || 'Not For Sale'}
          </p>
        </Card.Description>
      </Card.Content>
      <Card.Content extra style={{ textAlign: 'center' }}>
        {owner === currentAccount.address ? (
          <>
            <SetPrice course={course} setStatus={setStatus} />
            <TransferModal course={course} setStatus={setStatus} />
          </>
        ) : (
          <>
            <BuyCourse course={course} setStatus={setStatus} />
          </>
        )}
      </Card.Content>
    </Card>
  )
}

const CourseCards = props => {
  const { courses, setStatus } = props

  if (courses.length === 0) {
    return (
        <p>Mint courses</p>
    )
  }

  return (
    <Grid columns={3}>
      {courses.map((course, i) => (
        <Grid.Column key={`course-${i}`}>
          <CourseCard course={course} setStatus={setStatus} />
        </Grid.Column>
      ))}
    </Grid>
  )
}

export default CourseCards
