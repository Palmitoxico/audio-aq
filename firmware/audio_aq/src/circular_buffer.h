/*
 * Circular buffer header
 */

#include <stdint.h>

#ifndef RingBuffer_H_
#define RingBuffer_H_

typedef struct
{
	uint8_t data[64];
	int8_t size;
} EndPointBuffer;

typedef struct
{
	uint16_t start;
	uint16_t end;
	uint16_t size;
	EndPointBuffer *buffer;
} RingBuffer;

int RingBufferSpaceAvailable(RingBuffer* ringb);
int RingBufferSpaceUsed(RingBuffer* ringb);
void RingBufferInit(RingBuffer* ringb, EndPointBuffer *buffer, uint16_t bsize);
int RingBufferWrite(RingBuffer* ringb, EndPointBuffer *data);
int RingBufferRead(RingBuffer* ringb, EndPointBuffer *data);

#endif
